/* generate.c: each compound-text set's table, through libX11's lcUniConv. */
#include <stdio.h>
#include <stdlib.h>

typedef unsigned int ucs4_t;
typedef void *conv_t;
#define RET_ILSEQ 0
#define RET_TOOFEW(n) (-1 - (n))
#define RET_TOOSMALL -1

#include "iso8859_1.h"
#include "iso8859_2.h"
#include "iso8859_3.h"
#include "iso8859_4.h"
#include "iso8859_5.h"
#include "iso8859_6.h"
#include "iso8859_7.h"
#include "iso8859_8.h"
#include "iso8859_9.h"
#include "iso8859_10.h"
#include "iso8859_11.h"
#include "iso8859_13.h"
#include "iso8859_14.h"
#include "iso8859_15.h"
#include "iso8859_16.h"

typedef struct {
    unsigned short indx;
    unsigned short used;
} Summary16;

#include "gb2312.h"
#include "jisx0208.h"
#include "jisx0212.h"
#include "ksc5601.h"
#include "big5.h"
#include "big5hkscs.h"
#include "gbk.h"

typedef int (*decode)(conv_t, ucs4_t *, const unsigned char *, int);

static FILE *create(const char *name) {
    FILE *f = fopen(name, "wb");
    if (!f) { perror(name); exit(1); }
    return f;
}

static void finish(FILE *f, const char *name) {
    if (ferror(f) || fclose(f)) { perror(name); exit(1); }
}

/* Writes the bytes' character as little-endian UTF-16, 0 for none. */
static void entry(FILE *f, decode d, unsigned char b1, unsigned char b2, int n) {
    unsigned char s[2] = {b1, b2};
    ucs4_t wc = 0;
    if (d(NULL, &wc, s, n) != n) wc = 0;
    else if (wc == 0 || wc > 0xffff || (wc >= 0xd800 && wc <= 0xdfff)) {
        fprintf(stderr, "generate: %02X %02X decodes to U+%X, which no table entry holds\n", b1, b2, wc);
        exit(1);
    }
    fputc(wc & 0xff, f);
    fputc(wc >> 8, f);
}

/* Each 96-set's upper half, by final byte: ABCDFGHLMTVY_bf. */
static void upper(void) {
    decode sets[] = {iso8859_1_mbtowc, iso8859_2_mbtowc, iso8859_3_mbtowc, iso8859_4_mbtowc, iso8859_7_mbtowc,
                     iso8859_6_mbtowc, iso8859_8_mbtowc, iso8859_5_mbtowc, iso8859_9_mbtowc, iso8859_11_mbtowc,
                     iso8859_10_mbtowc, iso8859_13_mbtowc, iso8859_14_mbtowc, iso8859_15_mbtowc, iso8859_16_mbtowc};
    FILE *f = create("upper.bin");
    for (unsigned i = 0; i < sizeof sets / sizeof *sets; i++)
        for (int b = 0xa0; b <= 0xff; b++) entry(f, sets[i], b, 0, 1);
    finish(f, "upper.bin");
}

/* A 94 by 94 set as G0 holds it. */
static void grid(const char *name, decode d) {
    FILE *f = create(name);
    for (int b1 = 0x21; b1 <= 0x7e; b1++)
        for (int b2 = 0x21; b2 <= 0x7e; b2++) entry(f, d, b1, b2, 2);
    finish(f, name);
}

/* An extended segment's pairs: leads 0x81-0xFE, trails 0x40-0x7E, `high`-0xFE. */
static void pairs(const char *name, decode d, int high) {
    FILE *f = create(name);
    for (int b1 = 0x81; b1 <= 0xfe; b1++)
        for (int b2 = 0x40; b2 <= 0xfe; b2++)
            if (b2 <= 0x7e || b2 >= high) entry(f, d, b1, b2, 2);
    finish(f, name);
}

int main(void) {
    upper();
    grid("gb2312.bin", gb2312_mbtowc);
    grid("jisx0208.bin", jisx0208_mbtowc);
    grid("jisx0212.bin", jisx0212_mbtowc);
    grid("ksc5601.bin", ksc5601_mbtowc);
    pairs("big5.bin", big5_mbtowc, 0xa1);
    pairs("big5hkscs.bin", big5hkscs_mbtowc, 0xa1);
    pairs("gbk.bin", gbk_mbtowc, 0x80);
    return 0;
}
