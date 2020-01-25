#include <stdbool.h>
#include <string.h>
#ifdef _WIN32
#include <windows.h>
#else
#include <pthread.h>
#endif

#include "wrapper.h"

#ifdef _WIN32
static DWORD error_key;
#else
static pthread_key_t error_key;
#endif

typedef struct mupdf_error {
    int type;
    char* message;
} mupdf_error_t;

static void drop_tls_error(void *arg) {
    if (arg == NULL) {
        return;
    }
	mupdf_error_t* err = (mupdf_error_t*)arg;
    if (err->message != NULL) {
        free(err->message);
        err->message = NULL;
    }
    free(err);
}

static void init_tls_error_key() {
    if (!error_key) {
#ifdef _WIN32
        error_key = TlsAlloc();
        if (error_key == TLS_OUT_OF_INDEXES) {
            return NULL;
        }
#else
        pthread_key_create(&error_key, drop_tls_error);
#endif
    }
}

void mupdf_save_error(fz_context* ctx) {
    init_tls_error_key();
    int type = fz_caught(ctx);
    const char* message = fz_caught_message(ctx);
    mupdf_error_t* err = malloc(sizeof(mupdf_error_t));
    err->type = type;
    err->message = strdup(message);
#ifdef _WIN32
    TlsSetValue(error_key, err);
#else
    pthread_setspecific(error_key, err);
#endif
}

mupdf_error_t* mupdf_error() {
    if (!error_key) {
        return NULL;
    }
    mupdf_error_t* err = (mupdf_error_t*)
#ifdef _WIN32
    TlsGetValue(error_key);
#else
    pthread_getspecific(error_key);
#endif
    return err;
}

void mupdf_clear_error() {
    if (!error_key) {
        return;
    }
    mupdf_error_t* existing_err = mupdf_error();
    drop_tls_error(existing_err);

#ifdef _WIN32
    TlsSetValue(error_key, NULL);
#else
    pthread_setspecific(error_key, NULL);
#endif
}

fz_pixmap* mupdf_new_pixmap(fz_context* ctx, fz_colorspace* cs, int x, int y, int w, int h, bool alpha) {
    fz_pixmap *pixmap = NULL;
    fz_try(ctx) {
        pixmap = fz_new_pixmap(ctx, cs, w, h, NULL, alpha);
		pixmap->x = x;
		pixmap->y = y;
    }
    fz_catch(ctx) {
        mupdf_save_error(ctx);
    }
    return pixmap;
}

void mupdf_clear_pixmap(fz_context* ctx, fz_pixmap* pixmap) {
    fz_try(ctx) {
		fz_clear_pixmap(ctx, pixmap);
    }
	fz_catch(ctx) {
		mupdf_save_error(ctx);
    }
}