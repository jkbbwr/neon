#include "libneon_rt.h"

#include "internal.h"

#include <errno.h>
#include <fcntl.h>
#include <stdio.h>
#include <stdlib.h>
#include <sys/uio.h>
#include <unistd.h>

// The batch size for `writev`. `IOV_MAX` is only visible under feature-test macros we do
// not set, and a *smaller* batch is always valid -- the call just runs more than once -- so
// this pins the value POSIX guarantees Linux provides rather than probing for it.
#define NEON_IOV_MAX 1024

// A NUL-terminated copy of a `neon_str`, for the C APIs that demand one. `neon_str` is a
// length-delimited *view* -- a slice of a larger buffer is not terminated -- so this cannot
// be skipped by passing `.data`. Caller frees.
static char* neon_cstr(neon_str s) {
    char* p = (char*)malloc(s.len + 1);
    if (p == NULL) neon_trap("out of memory");
    if (s.len) memcpy(p, s.data, s.len);
    p[s.len] = 0;
    return p;
}
// ---- files ----
//
// Descriptors, not `FILE*`: `writev` wants one, and buffering here would only sit between
// the caller's iolist and the kernel.
//
// Failure travels as a *value*. Every fallible call returns `-errno`, and the ones that
// also return data use an out-parameter, which codegen turns into a Neon tuple (see
// `emit_native_out`). An earlier draft kept an errno-style flag in a static; any
// intervening call could clobber it and it said nothing at the call site.
//
// `neon_io_strerror` is a pure function of a code, so rendering a failure needs no state
// at all.

// `mode`: 0 read, 1 write (truncate), 2 append. The flags stay on this side because they
// are platform constants.
int64_t neon_io_open(neon_str path, int64_t mode) {
    char* p = neon_cstr(path);
    int flags = mode == 0   ? O_RDONLY
                : mode == 1 ? (O_WRONLY | O_CREAT | O_TRUNC)
                            : (O_WRONLY | O_CREAT | O_APPEND);
    int fd = open(p, flags, 0666);
    int64_t r = fd < 0 ? -(int64_t)errno : (int64_t)fd;
    free(p);
    neon_release(path.owner);
    return r;
}

// A bare descriptor: the armed flag that stops a double close lives in the `Resource`
// wrapping this, not here.
int64_t neon_io_close(int64_t fd) {
    return close((int)fd) != 0 ? -(int64_t)errno : 0;
}

neon_str neon_io_strerror(int64_t code) {
    const char* m = strerror(code < 0 ? (int)-code : (int)code);
    return neon_str_new(m, strlen(m));
}

// Everything left in the descriptor. `err` is the out-parameter: 0, or `-errno`. The data
// and the status come back together rather than through hidden state.
neon_str neon_io_read_all(int64_t fd, int64_t* err) {
    *err = 0;
    size_t cap = 4096, len = 0;
    char* buf = (char*)malloc(cap);
    if (buf == NULL) neon_trap("out of memory");
    for (;;) {
        if (len == cap) {
            cap *= 2;
            char* grown = (char*)realloc(buf, cap);
            if (grown == NULL) neon_trap("out of memory");
            buf = grown;
        }
        ssize_t got = read((int)fd, buf + len, cap - len);
        if (got < 0) {
            if (errno == EINTR) continue;
            *err = -(int64_t)errno;
            break;
        }
        if (got == 0) break;
        len += (size_t)got;
    }
    neon_str r = neon_str_new(buf, len);
    free(buf);
    return r;
}

// Write a list of `neon_str` views as one `writev`, so the pieces reach the kernel without
// ever being concatenated. `neon_str` is `{data, len, owner}` and `iovec` is
// `{iov_base, iov_len}` -- the first two fields line up, but the strides differ, so this
// copies pointer/length pairs and never a payload byte.
//
// Longer lists go in batches, and a short write resumes where it stopped rather than
// counting as failure -- that is the contract `writev` actually offers.
int64_t neon_io_writev(int64_t fd, neon_list* parts) {
    const neon_str* items = (const neon_str*)parts->data;
    int64_t status = 0;
    size_t i = 0;
    while (status == 0 && i < parts->len) {
        size_t batch = parts->len - i;
        if (batch > NEON_IOV_MAX) batch = NEON_IOV_MAX;
        struct iovec vec[NEON_IOV_MAX];
        size_t n = 0, total = 0;
        for (size_t j = 0; j < batch; j++) {
            if (items[i + j].len == 0) continue; // an empty piece is not a write
            vec[n].iov_base = items[i + j].data;
            vec[n].iov_len = items[i + j].len;
            total += items[i + j].len;
            n++;
        }
        i += batch;
        size_t done = 0, first = 0;
        while (done < total) {
            ssize_t got = writev((int)fd, vec + first, (int)(n - first));
            if (got < 0) {
                if (errno == EINTR) continue;
                status = -(int64_t)errno;
                break;
            }
            done += (size_t)got;
            size_t adv = (size_t)got;
            while (first < n && adv >= vec[first].iov_len) {
                adv -= vec[first].iov_len;
                first++;
            }
            if (first < n && adv) {
                vec[first].iov_base = (char*)vec[first].iov_base + adv;
                vec[first].iov_len -= adv;
            }
        }
    }
    neon_release((neon_header*)parts);
    return status;
}

int64_t neon_io_remove(neon_str path) {
    char* p = neon_cstr(path);
    int64_t r = remove(p) == 0 ? 0 : -(int64_t)errno;
    free(p);
    neon_release(path.owner);
    return r;
}

bool neon_io_exists(neon_str path) {
    char* p = neon_cstr(path);
    bool ok = access(p, F_OK) == 0;
    free(p);
    neon_release(path.owner);
    return ok;
}
