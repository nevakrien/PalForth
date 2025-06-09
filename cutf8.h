#ifndef CUTF8_H
#define CUTF8_H

/**
 * This header works with utf8 and tries to provide a similar ish API to what you have with ASCII
 * The goal is to allow for classic libc incremental parsing of taking 1 char at a time
 * 
 * 
 * note that if you are using reads not made by this libarary 
 * its generally a good idea to have your null terminators 4 bytes long to avoid acidental UB
 * since cutf8_length into cutf8_valid will result in UB on invalid inputs with uninilized memory
 * 
 * 
 * If you find a BUG report it please
 */

#ifdef __cplusplus
extern "C" {
#endif

#include <stdio.h>
#include <stddef.h>
#include <stdint.h>

#define CUTF8_MAX_BYTES 4
#define UTF8_ERROR (EOF - 1)

/* Classify UTF-8 lead byte and return expected total length, or 0 if invalid */
static int cutf8_length(unsigned char b0) {
    if ((b0 & 0x80) == 0x00) return 1;
    else if ((b0 & 0xE0) == 0xC0) return 2;
    else if ((b0 & 0xF0) == 0xE0) return 3;
    else if ((b0 & 0xF8) == 0xF0) return 4;
    return 0;
}

/* Decode codepoint and validate range */
static int cutf8_valid(const char *s, int len) {
    uint32_t cp;
    int i;

    if (len < 1 || len > 4)
        return 0;

    for (i = 1; i < len; ++i)
        if ((s[i] & 0xC0) != 0x80)
            return 0;

    if (len == 1) {
        cp = (unsigned char)s[0];
    } else if (len == 2) {
        cp = ((s[0] & 0x1F) << 6) | (s[1] & 0x3F);
        if (cp < 0x80) return 0; /* overlong */
    } else if (len == 3) {
        cp = ((s[0] & 0x0F) << 12) |
             ((s[1] & 0x3F) << 6) |
             (s[2] & 0x3F);
        if (cp < 0x800) return 0; /* overlong */
    } else {
        cp = ((s[0] & 0x07) << 18) |
             ((s[1] & 0x3F) << 12) |
             ((s[2] & 0x3F) << 6) |
             (s[3] & 0x3F);
        if (cp < 0x10000 || cp > 0x10FFFF)
            return 0;
    }

    if (cp >= 0xD800 && cp <= 0xDFFF) return 0;
    return 1;
}

/*validates an entire buffer*/
static int cutf8_valid_buff(const char *input, size_t size) {
    size_t i = 0;
    while (i < size) {
        int len = cutf8_length((unsigned char)input[i]);
        if (len == 0 || i + (size_t)len > size)
            return 0;
        if (!cutf8_valid(input + i, len))
            return 0;
        i += (size_t)len;
    }
    return 1;
}


/**
 * similar to get_c 
 * writes the char into out [which needs 4 bytes extra room]
 * and increments len acording to how much was written
 */
static int cutf8_get(FILE *file, char *out, size_t *len) {
    int i;
    int c = fgetc(file);
    if (c == EOF) return EOF;

    out[0] = (char)c;
    int total = cutf8_length((unsigned char)c);
    if (total == 0) return UTF8_ERROR;

    for (i = 1; i < total; ++i) {
        c = fgetc(file);
        if (c == EOF || (c & 0xC0) != 0x80) return UTF8_ERROR;
        out[i] = (char)c;
    }

    if (!cutf8_valid(out, total)) return UTF8_ERROR;
    *len += (size_t)total;
    return 0;
}

/**
 * similar to put_c 
 * reads the char from in
 * and increments len acording to how much was written to file
 */
static int cutf8_put(FILE *file, const char *input, size_t *size) {
    int i;
    int len = cutf8_length((unsigned char)input[0]);
    if (len == 0 || !cutf8_valid(input, len)) return UTF8_ERROR;

    for (i = 0; i < len; ++i) {
        if (fputc((unsigned char)input[i], file) == EOF)
            return EOF;
    }

    if (size) *size += (size_t)len;
    return 0;
}

/*copies a char from input to output*/
static size_t cutf8_copy(char *output,const char *input) {
    int i;
    char tmp[CUTF8_MAX_BYTES];
    int len = cutf8_length((unsigned char)input[0]);
    if (len == 0) return 0;

    for (i = 0; i < len; ++i) tmp[i] = input[i];
    if (!cutf8_valid(tmp, len)) return 0;
    for (i = 0; i < len; ++i) output[i] = tmp[i];

    return (size_t)len;
}

/**
 * skips forward 1 char assuming null termination
 * if the utf8 is not valid returns a NULL as well
*/
static char *cutf8_skip(const char *input) {
    int len, i;
    if (!input || input[0] == '\0') return NULL;

    len = cutf8_length((unsigned char)input[0]);
    if (len == 0) return NULL;

    for (i = 1; i < len; ++i)
        if ((input[i] & 0xC0) != 0x80)
            return NULL;

    if (!cutf8_valid(input, len)) return NULL;
    return (char *)(input + len);
}

#ifdef __cplusplus
}
#endif

#endif /* CUTF8_H */