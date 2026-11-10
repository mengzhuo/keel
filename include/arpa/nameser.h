/* Copyright (c) 2026 Meng Zhuo */
/* SPDX-License-Identifier: BSD-3-Clause */

#ifndef _ARPA_NAMESER_H
#define _ARPA_NAMESER_H

#ifdef __cplusplus
extern "C" {
#endif

int dn_expand(const unsigned char *, const unsigned char *, const unsigned char *, char *, int);

#ifdef __cplusplus
}
#endif


/* musl 1.2.6 compatibility macros */
#define ADD ns_uop_add
#define C_ANY ns_c_any
#define C_CHAOS ns_c_chaos
#define C_HS ns_c_hs
#define C_IN ns_c_in
#define C_NONE ns_c_none
#define DELETE ns_uop_delete
#define FORMERR ns_r_formerr
#define GETLONG NS_GET32
#define GETSHORT NS_GET16
#define HFIXEDSZ NS_HFIXEDSZ
#define IN6ADDRSZ NS_IN6ADDRSZ
#define INADDRSZ NS_INADDRSZ
#define INDIR_MASK NS_CMPRSFLGS
#define INT16SZ NS_INT16SZ
#define INT32SZ NS_INT32SZ
#define INT8SZ NS_INT8SZ
#define IQUERY ns_o_iquery
#define MAXCDNAME NS_MAXCDNAME
#define MAXDNAME NS_MAXDNAME
#define MAXLABEL NS_MAXLABEL
#define NAMESERVER_PORT NS_DEFAULTPORT
#define NOERROR ns_r_noerror
#define NOTAUTH ns_r_notauth
#define NOTIMP ns_r_notimpl
#define NOTZONE ns_r_notzone
#define NS_ALG_DH 2
#define NS_ALG_DSA 3
#define NS_ALG_DSS NS_ALG_DSA
#define NS_ALG_EXPIRE_ONLY 253
#define NS_ALG_MD5RSA 1
#define NS_ALG_PRIVATE_OID 254
#define NS_DSA_MAX_BYTES 405
#define NS_DSA_MIN_SIZE 213
#define NS_DSA_SIG_SIZE 41
#define NS_GET16(s, cp) (void)((s) = ns_get16(((cp)+=2)-2))
#define NS_GET32(l, cp) (void)((l) = ns_get32(((cp)+=4)-4))
#define NS_KEY_EXTENDED_FLAGS 0x1000
#define NS_KEY_NAME_ENTITY 0x0200
#define NS_KEY_NAME_RESERVED 0x0300
#define NS_KEY_NAME_TYPE 0x0300
#define NS_KEY_NAME_USER 0x0000
#define NS_KEY_NAME_ZONE 0x0100
#define NS_KEY_NO_AUTH 0x8000
#define NS_KEY_NO_CONF 0x4000
#define NS_KEY_PROT_ANY 255
#define NS_KEY_PROT_DNSSEC 3
#define NS_KEY_PROT_EMAIL 2
#define NS_KEY_PROT_IPSEC 4
#define NS_KEY_PROT_TLS 1
#define NS_KEY_RESERVED10 0x0020
#define NS_KEY_RESERVED11 0x0010
#define NS_KEY_RESERVED2 0x2000
#define NS_KEY_RESERVED4 0x0800
#define NS_KEY_RESERVED5 0x0400
#define NS_KEY_RESERVED8 0x0080
#define NS_KEY_RESERVED9 0x0040
#define NS_KEY_RESERVED_BITMASK ( NS_KEY_RESERVED2 | NS_KEY_RESERVED4 | NS_KEY_RESERVED5 | NS_KEY_RESERVED8 | NS_KEY_RESERVED9 | NS_KEY_RESERVED10 | NS_KEY_RESERVED11 )
#define NS_KEY_RESERVED_BITMASK2 0xFFFF
#define NS_KEY_SIGNATORYMASK 0x000F
#define NS_KEY_TYPEMASK 0xC000
#define NS_KEY_TYPE_AUTH_CONF 0x0000
#define NS_KEY_TYPE_AUTH_ONLY 0x4000
#define NS_KEY_TYPE_CONF_ONLY 0x8000
#define NS_KEY_TYPE_NO_KEY 0xC000
#define NS_MD5RSA_MAX_BASE64 (((NS_MD5RSA_MAX_BYTES+2)/3)*4)
#define NS_MD5RSA_MAX_BITS 4096
#define NS_MD5RSA_MAX_BYTES ((NS_MD5RSA_MAX_BITS+7/8)*2+3)
#define NS_MD5RSA_MAX_SIZE ((NS_MD5RSA_MAX_BITS+7)/8)
#define NS_MD5RSA_MIN_BITS 512
#define NS_MD5RSA_MIN_SIZE ((NS_MD5RSA_MIN_BITS+7)/8)
#define NS_NOTIFY_OP ns_o_notify
#define NS_NXT_BITS 8
#define NS_NXT_BIT_CLEAR(n, p) (p[(n)/NS_NXT_BITS] &= ~(0x80>>((n)%NS_NXT_BITS)))
#define NS_NXT_BIT_ISSET(n, p) (p[(n)/NS_NXT_BITS] & (0x80>>((n)%NS_NXT_BITS)))
#define NS_NXT_BIT_SET(n, p) (p[(n)/NS_NXT_BITS] |= (0x80>>((n)%NS_NXT_BITS)))
#define NS_NXT_MAX 127
#define NS_OPT_DNSSEC_OK 0x8000U
#define NS_OPT_NSID 3
#define NS_PUT16(s, cp) ns_put16((s), ((cp)+=2)-2)
#define NS_PUT32(l, cp) ns_put32((l), ((cp)+=4)-4)
#define NS_SIG_ALG 2
#define NS_SIG_EXPIR 8
#define NS_SIG_FOOT 16
#define NS_SIG_LABELS 3
#define NS_SIG_OTTL 4
#define NS_SIG_SIGNED 12
#define NS_SIG_SIGNER 18
#define NS_SIG_TYPE 0
#define NS_TSIG_ALG_HMAC_MD5 "HMAC-MD5.SIG-ALG.REG.INT"
#define NS_TSIG_ERROR_FORMERR -12
#define NS_TSIG_ERROR_NO_SPACE -11
#define NS_TSIG_ERROR_NO_TSIG -10
#define NS_TSIG_FUDGE 300
#define NS_TSIG_TCP_COUNT 100
#define NS_UPDATE_OP ns_o_update
#define NXDOMAIN ns_r_nxdomain
#define NXRRSET ns_r_nxrrset
#define PACKETSZ NS_PACKETSZ
#define PUTLONG NS_PUT32
#define PUTSHORT NS_PUT16
#define QFIXEDSZ NS_QFIXEDSZ
#define QUERY ns_o_query
#define REFUSED ns_r_refused
#define RRFIXEDSZ NS_RRFIXEDSZ
#define SERVFAIL ns_r_servfail
#define STATUS ns_o_status
#define S_ADDT ns_s_ar
#define S_PREREQ ns_s_pr
#define S_UPDATE ns_s_ud
#define S_ZONE ns_s_zn
#define T_A ns_t_a
#define T_A6 ns_t_a6
#define T_AAAA ns_t_aaaa
#define T_AFSDB ns_t_afsdb
#define T_ANY ns_t_any
#define T_ATMA ns_t_atma
#define T_AVC ns_t_avc
#define T_AXFR ns_t_axfr
#define T_CAA ns_t_caa
#define T_CDNSKEY ns_t_cdnskey
#define T_CDS ns_t_cds
#define T_CNAME ns_t_cname
#define T_CSYNC ns_t_csync
#define T_DHCID ns_t_dhcid
#define T_DLV ns_t_dlv
#define T_DNAME ns_t_dname
#define T_DNSKEY ns_t_dnskey
#define T_DS ns_t_ds
#define T_EID ns_t_eid
#define T_EUI48 ns_t_eui48
#define T_EUI64 ns_t_eui64
#define T_GID ns_t_gid
#define T_GPOS ns_t_gpos
#define T_HINFO ns_t_hinfo
#define T_HIP ns_t_hip
#define T_IPSECKEY ns_t_ipseckey
#define T_ISDN ns_t_isdn
#define T_IXFR ns_t_ixfr
#define T_KEY ns_t_key
#define T_L32 ns_t_l32
#define T_L64 ns_t_l64
#define T_LOC ns_t_loc
#define T_LP ns_t_lp
#define T_MAILA ns_t_maila
#define T_MAILB ns_t_mailb
#define T_MB ns_t_mb
#define T_MD ns_t_md
#define T_MF ns_t_mf
#define T_MG ns_t_mg
#define T_MINFO ns_t_minfo
#define T_MR ns_t_mr
#define T_MX ns_t_mx
#define T_NAPTR ns_t_naptr
#define T_NID ns_t_nid
#define T_NIMLOC ns_t_nimloc
#define T_NINFO ns_t_ninfo
#define T_NS ns_t_ns
#define T_NSAP ns_t_nsap
#define T_NSAP_PTR ns_t_nsap_ptr
#define T_NSEC ns_t_nsec
#define T_NSEC3 ns_t_nsec3
#define T_NSEC3PARAM ns_t_nsec3param
#define T_NULL ns_t_null
#define T_NXT ns_t_nxt
#define T_OPENPGPKEY ns_t_openpgpkey
#define T_PTR ns_t_ptr
#define T_PX ns_t_px
#define T_RKEY ns_t_rkey
#define T_RP ns_t_rp
#define T_RRSIG ns_t_rrsig
#define T_RT ns_t_rt
#define T_SIG ns_t_sig
#define T_SMIMEA ns_t_smimea
#define T_SOA ns_t_soa
#define T_SPF ns_t_spf
#define T_SRV ns_t_srv
#define T_SSHFP ns_t_sshfp
#define T_TA ns_t_ta
#define T_TALINK ns_t_talink
#define T_TKEY ns_t_tkey
#define T_TLSA ns_t_tlsa
#define T_TSIG ns_t_tsig
#define T_TXT ns_t_txt
#define T_UID ns_t_uid
#define T_UINFO ns_t_uinfo
#define T_UNSPEC ns_t_unspec
#define T_URI ns_t_uri
#define T_WKS ns_t_wks
#define T_X25 ns_t_x25
#define YXDOMAIN ns_r_yxdomain
#define YXRRSET ns_r_yxrrset
#define ns_t_mrr_p(t) ((t) == ns_t_tsig || (t) == ns_t_opt)
#define ns_t_qt_p(t) (ns_t_xfr_p(t) || (t) == ns_t_any || (t) == ns_t_mailb || (t) == ns_t_maila)
#define ns_t_rr_p(t) (!ns_t_qt_p(t) && !ns_t_mrr_p(t))
#define ns_t_udp_p(t) ((t) != ns_t_axfr && (t) != ns_t_zxfr)
#define ns_t_xfr_p(t) ((t) == ns_t_axfr || (t) == ns_t_ixfr || (t) == ns_t_zxfr)

/* musl 1.2.6 compatibility macros */
#define __BIND 19950621
#define __NAMESER 19991006

#include <stddef.h>
#include <stdint.h>

#define NS_PACKETSZ     512
#define NS_MAXDNAME     1025
#define NS_MAXMSG       65535
#define NS_MAXCDNAME    255
#define NS_MAXLABEL     63
#define NS_HFIXEDSZ     12
#define NS_QFIXEDSZ     4
#define NS_RRFIXEDSZ    10
#define NS_INT32SZ      4
#define NS_INT16SZ      2
#define NS_INT8SZ       1
#define NS_INADDRSZ     4
#define NS_IN6ADDRSZ    16
#define NS_CMPRSFLGS    0xc0
#define NS_DEFAULTPORT  53

typedef enum __ns_type {
	ns_t_invalid = 0,
	ns_t_a = 1,
	ns_t_ns = 2,
	ns_t_md = 3,
	ns_t_mf = 4,
	ns_t_cname = 5,
	ns_t_soa = 6,
	ns_t_mb = 7,
	ns_t_mg = 8,
	ns_t_mr = 9,
	ns_t_null = 10,
	ns_t_wks = 11,
	ns_t_ptr = 12,
	ns_t_hinfo = 13,
	ns_t_minfo = 14,
	ns_t_mx = 15,
	ns_t_txt = 16,
	ns_t_rp = 17,
	ns_t_afsdb = 18,
	ns_t_x25 = 19,
	ns_t_isdn = 20,
	ns_t_rt = 21,
	ns_t_nsap = 22,
	ns_t_nsap_ptr = 23,
	ns_t_sig = 24,
	ns_t_key = 25,
	ns_t_px = 26,
	ns_t_gpos = 27,
	ns_t_aaaa = 28,
	ns_t_loc = 29,
	ns_t_nxt = 30,
	ns_t_eid = 31,
	ns_t_nimloc = 32,
	ns_t_srv = 33,
	ns_t_atma = 34,
	ns_t_naptr = 35,
	ns_t_kx = 36,
	ns_t_cert = 37,
	ns_t_a6 = 38,
	ns_t_dname = 39,
	ns_t_sink = 40,
	ns_t_opt = 41,
	ns_t_apl = 42,
	ns_t_ds = 43,
	ns_t_sshfp = 44,
	ns_t_ipseckey = 45,
	ns_t_rrsig = 46,
	ns_t_nsec = 47,
	ns_t_dnskey = 48,
	ns_t_dhcid = 49,
	ns_t_nsec3 = 50,
	ns_t_nsec3param = 51,
	ns_t_tlsa = 52,
	ns_t_smimea = 53,
	ns_t_hip = 55,
	ns_t_ninfo = 56,
	ns_t_rkey = 57,
	ns_t_talink = 58,
	ns_t_cds = 59,
	ns_t_cdnskey = 60,
	ns_t_openpgpkey = 61,
	ns_t_csync = 62,
	ns_t_spf = 99,
	ns_t_uinfo = 100,
	ns_t_uid = 101,
	ns_t_gid = 102,
	ns_t_unspec = 103,
	ns_t_nid = 104,
	ns_t_l32 = 105,
	ns_t_l64 = 106,
	ns_t_lp = 107,
	ns_t_eui48 = 108,
	ns_t_eui64 = 109,
	ns_t_tkey = 249,
	ns_t_tsig = 250,
	ns_t_ixfr = 251,
	ns_t_axfr = 252,
	ns_t_mailb = 253,
	ns_t_maila = 254,
	ns_t_any = 255,
	ns_t_zxfr = 256,
	ns_t_uri = 256,
	ns_t_caa = 257,
	ns_t_avc = 258,
	ns_t_ta = 32768,
	ns_t_dlv = 32769,
	ns_t_max = 65536
} ns_type;

typedef enum __ns_sect {
    ns_s_qd = 0,
    ns_s_zn = 0,
    ns_s_an = 1,
    ns_s_pr = 1,
    ns_s_ns = 2,
    ns_s_ud = 2,
    ns_s_ar = 3,
    ns_s_max = 4
} ns_sect;

typedef struct __ns_msg {
    const unsigned char *_msg, *_eom;
    uint16_t _id, _flags, _counts[ns_s_max];
    const unsigned char *_sections[ns_s_max];
    ns_sect _sect;
    int _rrnum;
    const unsigned char *_msg_ptr;
} ns_msg;

struct _ns_flagdata { int mask, shift; };
extern const struct _ns_flagdata _ns_flagdata[];

#define ns_msg_id(handle) ((handle)._id + 0)
#define ns_msg_base(handle) ((handle)._msg + 0)
#define ns_msg_end(handle) ((handle)._eom + 0)
#define ns_msg_size(handle) ((handle)._eom - (handle)._msg)
#define ns_msg_count(handle, section) ((handle)._counts[section] + 0)
#define ns_msg_getflag(handle, flag) \
    (((handle)._flags & _ns_flagdata[flag].mask) >> _ns_flagdata[flag].shift)

typedef struct __ns_rr {
    char name[NS_MAXDNAME];
    uint16_t type;
    uint16_t rr_class;
    uint32_t ttl;
    uint16_t rdlength;
    const unsigned char *rdata;
} ns_rr;

#define ns_rr_name(rr)  (((rr).name[0] != '\0') ? (rr).name : ".")
#define ns_rr_type(rr)  ((ns_type)((rr).type + 0))
#define ns_rr_class(rr) ((ns_class)((rr).rr_class + 0))
#define ns_rr_ttl(rr)   ((rr).ttl + 0)
#define ns_rr_rdlen(rr) ((rr).rdlength + 0)
#define ns_rr_rdata(rr) ((rr).rdata + 0)

typedef enum __ns_flag {
    ns_f_qr, ns_f_opcode, ns_f_aa, ns_f_tc, ns_f_rd, ns_f_ra,
    ns_f_z, ns_f_ad, ns_f_cd, ns_f_rcode, ns_f_max
} ns_flag;

unsigned ns_get16(const unsigned char *);
unsigned long ns_get32(const unsigned char *);
void ns_put16(unsigned, unsigned char *);
void ns_put32(unsigned long, unsigned char *);
int ns_initparse(const unsigned char *, int, ns_msg *);
int ns_skiprr(const unsigned char *, const unsigned char *, ns_sect, int);
int ns_parserr(ns_msg *, ns_sect, int, ns_rr *);
int ns_name_uncompress(const unsigned char *, const unsigned char *, const unsigned char *, char *, size_t);


/* musl 1.2.6 per-header compatibility macros */
#define __LONG_MAX 0x7fffffffffffffffL
#define __LITTLE_ENDIAN 1234
#define __BYTE_ORDER 1234
#define __BIG_ENDIAN 4321
#endif
