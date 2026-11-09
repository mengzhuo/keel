#!/usr/bin/env python3
"""Check that public header macros agree with crabc's Rust constants.

Every Rust numeric constant that also exists as an object-like macro in the
crabc headers is evaluated (with header macros resolved recursively) and
compared numerically.  This catches a header being updated with a musl value
that the Rust implementation does not actually use.

Constants whose Rust expression is not a simple integer expression are
reported as skipped, as are macros that expand to non-integer tokens.  `SB` is
ignored because it is an internal character-range constant unrelated to the
ELF `SB` macro.
"""

import os, re, sys, subprocess, collections

ROOT=os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
ARCHS=['x86_64','aarch64','riscv64']
CC={'x86_64':'gcc','aarch64':'aarch64-linux-gnu-gcc','riscv64':'riscv64-linux-gnu-gcc'}
INCLUDE=os.path.join(ROOT,'include')


TOK=re.compile(r'0[xX][0-9a-fA-F]+[uUlL]*|0[0-7]+[uUlL]*|\d+[uUlL]*|[A-Za-z_]\w*|<<|>>|<=|>=|==|!=|&&|\|\||[-+*/%~!&|^()<>]')

def toks(expr):
    return TOK.findall(expr)

def parse_int(tok):
    t=tok.rstrip('uUlL')
    if t.lower().startswith('0x'): return int(t,16)
    if len(t)>1 and t.startswith('0'): return int(t,8)
    return int(t,10)

class Bad(Exception): pass

class Parser:
    def __init__(self, toks, resolve):
        self.t=toks; self.i=0; self.resolve=resolve
    def peek(self): return self.t[self.i] if self.i<len(self.t) else None
    def take(self):
        v=self.peek(); self.i+=1; return v
    def parse(self):
        v=self.or_()
        if self.i!=len(self.t): raise Bad('trailing')
        return v
    def or_(self):
        v=self.xor()
        while self.peek()=='|': self.take(); v=v|self.xor()
        return v
    def xor(self):
        v=self.and_()
        while self.peek()=='^': self.take(); v=v^self.and_()
        return v
    def and_(self):
        v=self.shift()
        while self.peek()=='&': self.take(); v=v&self.shift()
        return v
    def shift(self):
        v=self.add()
        while self.peek() in ('<<','>>'):
            op=self.take(); n=self.add(); v=(v<<n) if op=='<<' else (v>>n)
        return v
    def add(self):
        v=self.mul()
        while self.peek() in ('+','-'):
            op=self.take(); n=self.mul(); v=v+n if op=='+' else v-n
        return v
    def mul(self):
        v=self.unary()
        while self.peek() in ('*','/','%'):
            op=self.take(); n=self.unary()
            if op=='*': v=v*n
            elif op=='/':
                if n==0: raise Bad('div0')
                q=abs(v)//abs(n); v=q if (v<0)==(n<0) else -q
            else:
                if n==0: raise Bad('mod0')
                v=v-int(v/n)*n if False else v % n
        return v
    def unary(self):
        p=self.peek()
        if p=='+': self.take(); return self.unary()
        if p=='-': self.take(); return -self.unary()
        if p=='~': self.take(); return ~self.unary()
        if p=='!': self.take(); return int(not self.unary())
        return self.atom()
    def atom(self):
        if self.peek()=='(':
            self.take(); v=self.or_()
            if self.take()!=')': raise Bad('paren')
            return v
        x=self.take()
        if x is None: raise Bad('eof')
        if re.match(r'^[0-9]',x): return parse_int(x)
        if re.match(r'^[A-Za-z_]',x): return self.resolve(x)
        raise Bad('token '+x)

def eval_c(expr, table, seen=()):
    def resolve(name):
        if name in seen: raise Bad('recursive')
        if name not in table: raise Bad('missing '+name)
        ent=table[name]
        if ent['kind']!='object': raise Bad('function '+name)
        return eval_c(ent['body'], table, seen+(name,))
    return Parser(toks(expr), resolve).parse()

def eval_rust(expr):
    e=expr.strip()
    e=re.sub(r'_(?=[0-9])','',e)
    e=re.sub(r'\b(?:u|i)(?:8|16|32|64|size)\b','',e)
    repl={'i64::MAX':'9223372036854775807','i64::MIN':'-9223372036854775808',
          'i32::MAX':'2147483647','i32::MIN':'-2147483648',
          'u64::MAX':'18446744073709551615','u32::MAX':'4294967295',
          'usize::MAX':'18446744073709551615','isize::MAX':'9223372036854775807',
          'isize::MIN':'-9223372036854775808'}
    for k,v in repl.items(): e=e.replace(k,v)
    if '::' in e or ' as ' in e or 'sizeof' in e: raise Bad('unsupported')
    def resolve(name): raise Bad('ident '+name)
    return Parser(toks(e), resolve).parse()

def all_headers_source():
    headers=[]
    for root,_,fs in os.walk(INCLUDE):
        for f in fs:
            if not f.endswith('.h'): continue
            rel=os.path.relpath(os.path.join(root,f),INCLUDE)
            if rel.startswith('bits/'): continue
            headers.append(rel)
    lines=['#define _GNU_SOURCE 1']+[f'#include <{h}>' for h in sorted(headers)]
    return '\n'.join(lines)+'\n'

def macro_table(arch, source):
    p=subprocess.run([CC[arch],'-std=gnu11','-D_GNU_SOURCE','-nostdinc','-I',INCLUDE,'-dM','-E','-x','c','-'],input=source,capture_output=True,text=True)
    if p.returncode: raise SystemExit(p.stderr)
    tab={}
    for line in p.stdout.splitlines():
        if not line.startswith('#define '): continue
        m=re.match(r'([A-Za-z_]\w*)(?:\(([^)]*)\))?(?:\s+(.*))?$', line[8:])
        if not m: continue
        name,params,body=m.groups()
        tab[name]={'kind':'object' if params is None else 'function','body':body or ''}
    return tab

def rust_constants():
    # returns name -> arch -> value ; None arch means all
    out=collections.defaultdict(lambda: collections.defaultdict(set))
    for base in ['libc/src','ldso/src']:
        for root,_,fs in os.walk(os.path.join(ROOT,base)):
            for f in fs:
                if not f.endswith('.rs'): continue
                path=os.path.join(root,f)
                text=open(path,errors='ignore').read().splitlines()
                depth=0; stack=[]; pending=None
                block_comment=False
                for line in text:
                    s=line.strip()
                    if block_comment:
                        if '*/' in s: block_comment=False
                        continue
                    if s.startswith('/*'):
                        if '*/' not in s: block_comment=True
                        continue
                    cm=re.match(r'#\[cfg\(target_arch\s*=\s*"(\w+)"\)\]',s)
                    if cm: pending=cm.group(1)
                    # only handle simple mod blocks
                    mm=re.match(r'(?:pub\s+)?mod\s+\w+\s*\{',s)
                    if mm:
                        stack.append((depth,pending)); pending=None
                    elif not s.startswith('#[') and not s.startswith('//'):
                        # pending cfg applies to next item; keep for const too
                        pass
                    c=re.match(r'(pub\s+)?const\s+([A-Z][A-Z0-9_]*)\s*:\s*[^=]+=\s*([^;]+);',s)
                    if c:
                        arch=pending if pending else (stack[-1][1] if stack else None)

                        out[c.group(2)][arch].add(c.group(3).strip())
                        pending=None
                    # brace depth
                    depth += s.count('{')-s.count('}')
                    while stack and depth<=stack[-1][0]:
                        stack.pop()
    return out

if __name__ == '__main__':
    import argparse
    parser = argparse.ArgumentParser()
    parser.add_argument('--include', default=INCLUDE)
    parser.add_argument('--min-compared', type=int, default=1300)
    parser.add_argument('--verbose', action='store_true')
    args = parser.parse_args()
    INCLUDE = os.path.abspath(args.include)
    source = all_headers_source()
    rc = rust_constants()
    total = matches = 0
    mism = []
    skipped = collections.Counter()
    for arch in ARCHS:
        tab = macro_table(arch, source)
        for name, byarch in rc.items():
            vals = set()
            if arch in byarch:
                vals |= byarch[arch]
            if None in byarch:
                vals |= byarch[None]
            if not vals:
                continue
            if len(vals) > 1:
                skipped['rust-conflict'] += 1
                continue
            if name in ('SB',):
                continue
            if name not in tab:
                continue
            ent = tab[name]
            if ent['kind'] != 'object':
                skipped['function-macro'] += 1
                continue
            try:
                rv = eval_rust(next(iter(vals)))
            except Bad:
                skipped['rust-expr'] += 1
                continue
            try:
                cv = eval_c(ent['body'], tab)
            except Bad:
                skipped['c-expr'] += 1
                continue
            total += 1
            if rv == cv:
                matches += 1
            else:
                mism.append((arch, name, next(iter(vals)), ent['body'], rv, cv))

        # musl exposes syscall numbers as __NR_name while crabc's Rust code
        # uses SYS_NAME.  Check that the two spellings agree too.
        for name, ent in tab.items():
            if not name.startswith('__NR_') or ent['kind'] != 'object':
                continue
            rust_name = 'SYS_' + name[len('__NR_'):].upper()
            vals = set()
            if arch in rc.get(rust_name, {}):
                vals |= rc[rust_name][arch]
            if None in rc.get(rust_name, {}):
                vals |= rc[rust_name][None]
            if not vals or len(vals) != 1:
                continue
            try:
                rv = eval_rust(next(iter(vals)))
                cv = eval_c(ent['body'], tab)
            except Bad:
                skipped['syscall-expr'] += 1
                continue
            total += 1
            if rv == cv:
                matches += 1
            else:
                mism.append((arch, name, next(iter(vals)), ent['body'], rv, cv))
    print(f'compared={total} matches={matches} mismatches={len(mism)} skipped={dict(skipped)}')
    for item in mism:
        print('  mismatch:', item)
    if total < args.min_compared:
        print(f'error: only {total} constants compared (expected at least {args.min_compared})')
    sys.exit(1 if mism or total < args.min_compared else 0)
