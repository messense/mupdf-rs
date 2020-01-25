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

void mupdf_save_error(fz_context* ctx) {
    init_tls_error_key();

    mupdf_error_t* existing_err = mupdf_error();
    drop_tls_error(existing_err);

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

/* Pixmap */
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

void mupdf_clear_pixmap_with_value(fz_context* ctx, fz_pixmap* pixmap, int value) {
    fz_try(ctx) {
        fz_clear_pixmap_with_value(ctx, pixmap, value);
    }
    fz_catch(ctx) {
        mupdf_save_error(ctx);
    }
}

void mupdf_save_pixmap_as_png(fz_context* ctx, fz_pixmap* pixmap, const char* filename) {
    fz_try(ctx) {
        fz_save_pixmap_as_png(ctx, pixmap, filename);
    }
    fz_catch(ctx) {
        mupdf_save_error(ctx);
    }
}

void mupdf_invert_pixmap(fz_context* ctx, fz_pixmap* pixmap) {
    fz_try(ctx) {
        fz_invert_pixmap(ctx, pixmap);
    }
    fz_catch(ctx) {
        mupdf_save_error(ctx);
    }
}

void mupdf_gamma_pixmap(fz_context* ctx, fz_pixmap* pixmap, float gamma) {
    fz_try(ctx) {
        fz_gamma_pixmap(ctx, pixmap, gamma);
    }
    fz_catch(ctx) {
        mupdf_save_error(ctx);
    }
}

/* Font */
fz_font* mupdf_new_font(fz_context* ctx, const char* name, int index) {
    fz_font* font = NULL;
    fz_try(ctx) {
        const unsigned char *data;
        int size;
        
        data = fz_lookup_base14_font(ctx, name, &size);
		if (data)
			font = fz_new_font_from_memory(ctx, name, data, size, index, 0);
		else
			font = fz_new_font_from_file(ctx, name, name, index, 0);
    }
    fz_catch(ctx) {
        mupdf_save_error(ctx);
    }
    return font;
}

int mupdf_encode_character(fz_context* ctx, fz_font* font, int unicode) {
    int glyph = 0;
    fz_try(ctx) {
        glyph = fz_encode_character(ctx, font, unicode);
    }
    fz_catch(ctx) {
        mupdf_save_error(ctx);
    }
    return glyph;
}

float mupdf_advance_glyph(fz_context* ctx, fz_font* font, int glyph, bool wmode) {
    float advance = 0;
    fz_try(ctx) {
        advance = fz_advance_glyph(ctx, font, glyph, wmode);
    }
    fz_catch(ctx) {
        mupdf_save_error(ctx);
    }
    return advance;
}

/* Image */
fz_image* mupdf_new_image_from_pixmap(fz_context* ctx, fz_pixmap* pixmap) {
    fz_image* image = NULL;
    fz_try(ctx) {
        image = fz_new_image_from_pixmap(ctx, pixmap, NULL);
    }
    fz_catch(ctx) {
        mupdf_save_error(ctx);
    }
    return image;
}

fz_image* mupdf_new_image_from_file(fz_context* ctx, const char* filename) {
    fz_image* image = NULL;
    fz_try(ctx) {
        image = fz_new_image_from_file(ctx, filename);
    }
    fz_catch(ctx) {
        mupdf_save_error(ctx);
    }
    return image;
}

fz_pixmap* mupdf_get_pixmap_from_image(fz_context* ctx, fz_image* image) {
    fz_pixmap* pixmap = NULL;
    fz_try(ctx) {
        pixmap = fz_get_pixmap_from_image(ctx, image, NULL, NULL, NULL, NULL);
    }
    fz_catch(ctx) {
        mupdf_save_error(ctx);
    }
    return pixmap;
}