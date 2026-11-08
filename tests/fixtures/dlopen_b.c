int who(void) { return 2; }

__thread int b_tls = 7;

int b_tls_get(void) { return b_tls; }
