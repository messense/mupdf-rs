#include <stdint.h>
#include <stdbool.h>
#include <string.h>
#include <assert.h>
#ifdef _WIN32
#include <windows.h>
#else
#include <pthread.h>
#endif

#include "wrapper.h"

/* Put the fz_context in thread-local storage */

#ifdef _WIN32
static CRITICAL_SECTION mutexes[FZ_LOCK_MAX];
#else
static pthread_mutex_t mutexes[FZ_LOCK_MAX];
#endif

static void lock(void *user, int lock)
{
    // supress unused variable warning
    (void)user;
#ifdef _WIN32
    EnterCriticalSection(&mutexes[lock]);
#else
    (void)pthread_mutex_lock(&mutexes[lock]);
#endif
}

static void unlock(void *user, int lock)
{
    // supress unused variable warning
    (void)user;
#ifdef _WIN32
    LeaveCriticalSection(&mutexes[lock]);
#else
    (void)pthread_mutex_unlock(&mutexes[lock]);
#endif
}

static const fz_locks_context locks =
    {
        NULL, /* user */
        lock,
        unlock};

typedef struct mupdf_error
{
    int type;
    char *message;
} mupdf_error_t;

static void mupdf_save_error(fz_context *ctx, mupdf_error_t **errptr)
{
    assert(errptr != NULL);
    int type = fz_caught(ctx);
    const char *message = fz_caught_message(ctx);
    mupdf_error_t *err = malloc(sizeof(mupdf_error_t));
    err->type = type;
    err->message = strdup(message);
    *errptr = err;
}

static mupdf_error_t *mupdf_new_error_from_str(const char *message) {
    mupdf_error_t *err = malloc(sizeof(mupdf_error_t));
    err->type = -1;
    err->message = strdup(message);
    return err;
}

void mupdf_drop_error(mupdf_error_t *err)
{
    if (err == NULL)
    {
        return;
    }
    if (err->message != NULL)
    {
        free(err->message);
    }
    free(err);
    err = NULL;
}

void mupdf_drop_str(char *s)
{
    if (s != NULL)
    {
        free(s);
        s = NULL;
    }
}

/* Context */
fz_context *mupdf_new_base_context()
{
    for (int i = 0; i < FZ_LOCK_MAX; i++)
    {
#ifdef _WIN32
        InitializeCriticalSection(&mutexes[i]);
#else
        (void)pthread_mutex_init(&mutexes[i], NULL);
#endif
    }
    fz_context *ctx = fz_new_context(NULL, &locks, FZ_STORE_DEFAULT);
    if (!ctx)
    {
        return NULL;
    }
    fz_register_document_handlers(ctx);
    return ctx;
}

void mupdf_drop_base_context(fz_context *ctx)
{
    for (int i = 0; i < FZ_LOCK_MAX; i++)
    {
#ifdef _WIN32
        DeleteCriticalSection(&mutexes[i]);
#else
        (void)pthread_mutex_destroy(&mutexes[i]);
#endif
    }

    fz_drop_context(ctx);
    ctx = NULL;
}

/* Pixmap */
fz_pixmap *mupdf_new_pixmap(fz_context *ctx, fz_colorspace *cs, int x, int y, int w, int h, bool alpha, mupdf_error_t **errptr)
{
    fz_pixmap *pixmap = NULL;
    fz_try(ctx)
    {
        pixmap = fz_new_pixmap(ctx, cs, w, h, NULL, alpha);
        pixmap->x = x;
        pixmap->y = y;
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return pixmap;
}

void mupdf_clear_pixmap(fz_context *ctx, fz_pixmap *pixmap, mupdf_error_t **errptr)
{
    fz_try(ctx)
    {
        fz_clear_pixmap(ctx, pixmap);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
}

void mupdf_clear_pixmap_with_value(fz_context *ctx, fz_pixmap *pixmap, int value, mupdf_error_t **errptr)
{
    fz_try(ctx)
    {
        fz_clear_pixmap_with_value(ctx, pixmap, value);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
}

void mupdf_save_pixmap_as_png(fz_context *ctx, fz_pixmap *pixmap, const char *filename, mupdf_error_t **errptr)
{
    fz_try(ctx)
    {
        fz_save_pixmap_as_png(ctx, pixmap, filename);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
}

void mupdf_invert_pixmap(fz_context *ctx, fz_pixmap *pixmap, mupdf_error_t **errptr)
{
    fz_try(ctx)
    {
        fz_invert_pixmap(ctx, pixmap);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
}

void mupdf_gamma_pixmap(fz_context *ctx, fz_pixmap *pixmap, float gamma, mupdf_error_t **errptr)
{
    fz_try(ctx)
    {
        fz_gamma_pixmap(ctx, pixmap, gamma);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
}

/* Font */
fz_font *mupdf_new_font(fz_context *ctx, const char *name, int index, mupdf_error_t **errptr)
{
    fz_font *font = NULL;
    fz_try(ctx)
    {
        const unsigned char *data;
        int size;

        data = fz_lookup_base14_font(ctx, name, &size);
        if (data)
        {
            font = fz_new_font_from_memory(ctx, name, data, size, index, 0);
        }
        else
        {
            font = fz_new_font_from_file(ctx, name, name, index, 0);
        }
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return font;
}

fz_font *mupdf_new_font_from_memory(fz_context *ctx, const char *name, int index, const unsigned char *data, int data_len, mupdf_error_t **errptr)
{
    fz_font *font = NULL;
    fz_try(ctx)
    {
        font = fz_new_font_from_memory(ctx, name, data, data_len, index, 0);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return font;
}

int mupdf_encode_character(fz_context *ctx, fz_font *font, int unicode, mupdf_error_t **errptr)
{
    int glyph = 0;
    fz_try(ctx)
    {
        glyph = fz_encode_character(ctx, font, unicode);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return glyph;
}

float mupdf_advance_glyph(fz_context *ctx, fz_font *font, int glyph, bool wmode, mupdf_error_t **errptr)
{
    float advance = 0;
    fz_try(ctx)
    {
        advance = fz_advance_glyph(ctx, font, glyph, wmode);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return advance;
}

/* Image */
fz_image *mupdf_new_image_from_pixmap(fz_context *ctx, fz_pixmap *pixmap, mupdf_error_t **errptr)
{
    fz_image *image = NULL;
    fz_try(ctx)
    {
        image = fz_new_image_from_pixmap(ctx, pixmap, NULL);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return image;
}

fz_image *mupdf_new_image_from_file(fz_context *ctx, const char *filename, mupdf_error_t **errptr)
{
    fz_image *image = NULL;
    fz_try(ctx)
    {
        image = fz_new_image_from_file(ctx, filename);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return image;
}

fz_pixmap *mupdf_get_pixmap_from_image(fz_context *ctx, fz_image *image, mupdf_error_t **errptr)
{
    fz_pixmap *pixmap = NULL;
    fz_try(ctx)
    {
        pixmap = fz_get_pixmap_from_image(ctx, image, NULL, NULL, NULL, NULL);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return pixmap;
}

/* Text */
fz_text *mupdf_new_text(fz_context *ctx, mupdf_error_t **errptr)
{
    fz_text *text = NULL;
    fz_try(ctx)
    {
        text = fz_new_text(ctx);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return text;
}

/* StrokeState */
fz_stroke_state *mupdf_new_stroke_state(
    fz_context *ctx, uint32_t start_cap, uint32_t dash_cap, uint32_t end_cap, uint32_t line_join, float line_width,
    float miter_limit, float dash_phase, const float dash[], int dash_len, mupdf_error_t **errptr)
{
    fz_stroke_state *stroke = NULL;
    fz_try(ctx)
    {
        stroke = fz_new_stroke_state_with_dash_len(ctx, dash_len);
        stroke->start_cap = start_cap;
        stroke->dash_cap = dash_cap;
        stroke->end_cap = end_cap;
        stroke->linejoin = line_join;
        stroke->linewidth = line_width;
        stroke->miterlimit = miter_limit;
        stroke->dash_phase = dash_phase;
        stroke->dash_len = dash_len;
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    memcpy(stroke->dash_list, dash, dash_len);
    return stroke;
}

fz_rect mupdf_bound_text(fz_context *ctx, fz_text *text, fz_stroke_state *stroke, fz_matrix ctm, mupdf_error_t **errptr)
{
    fz_rect rect;
    fz_try(ctx)
    {
        rect = fz_bound_text(ctx, text, stroke, ctm);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return rect;
}

/* Path */
fz_path *mupdf_new_path(fz_context *ctx, mupdf_error_t **errptr)
{
    fz_path *path = NULL;
    fz_try(ctx)
    {
        path = fz_new_path(ctx);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return path;
}

fz_path *mupdf_clone_path(fz_context *ctx, fz_path *old_path, mupdf_error_t **errptr)
{
    fz_path *path = NULL;
    fz_try(ctx)
    {
        path = fz_clone_path(ctx, old_path);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return path;
}

fz_point mupdf_currentpoint(fz_context *ctx, fz_path *path, mupdf_error_t **errptr)
{
    fz_point point;
    fz_try(ctx)
    {
        point = fz_currentpoint(ctx, path);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return point;
}

void mupdf_moveto(fz_context *ctx, fz_path *path, float x, float y, mupdf_error_t **errptr)
{
    fz_try(ctx)
    {
        fz_moveto(ctx, path, x, y);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
}

void mupdf_lineto(fz_context *ctx, fz_path *path, float x, float y, mupdf_error_t **errptr)
{
    fz_try(ctx)
    {
        fz_lineto(ctx, path, x, y);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
}

void mupdf_closepath(fz_context *ctx, fz_path *path, mupdf_error_t **errptr)
{
    fz_try(ctx)
    {
        fz_closepath(ctx, path);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
}

void mupdf_rectto(fz_context *ctx, fz_path *path, int x1, int y1, int x2, int y2, mupdf_error_t **errptr)
{
    fz_try(ctx)
    {
        fz_rectto(ctx, path, x1, y1, x2, y2);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
}

void mupdf_curveto(fz_context *ctx, fz_path *path, float cx1, float cy1, float cx2, float cy2, float ex, float ey, mupdf_error_t **errptr)
{
    fz_try(ctx)
    {
        fz_curveto(ctx, path, cx1, cy1, cx2, cy2, ex, ey);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
}

void mupdf_curvetov(fz_context *ctx, fz_path *path, float cx, float cy, float ex, float ey, mupdf_error_t **errptr)
{
    fz_try(ctx)
    {
        fz_curvetov(ctx, path, cx, cy, ex, ey);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
}

void mupdf_curvetoy(fz_context *ctx, fz_path *path, float cx, float cy, float ex, float ey, mupdf_error_t **errptr)
{
    fz_try(ctx)
    {
        fz_curvetoy(ctx, path, cx, cy, ex, ey);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
}

void mupdf_transform_path(fz_context *ctx, fz_path *path, fz_matrix ctm, mupdf_error_t **errptr)
{
    fz_try(ctx)
    {
        fz_transform_path(ctx, path, ctm);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
}

fz_rect mupdf_bound_path(fz_context *ctx, fz_path *path, fz_stroke_state *stroke, fz_matrix ctm, mupdf_error_t **errptr)
{
    fz_rect rect;
    fz_try(ctx)
    {
        rect = fz_bound_path(ctx, path, stroke, ctm);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return rect;
}

/* Page */
fz_rect mupdf_bound_page(fz_context *ctx, fz_page *page, mupdf_error_t **errptr)
{
    fz_rect rect;
    fz_try(ctx)
    {
        rect = fz_bound_page(ctx, page);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return rect;
}

fz_pixmap *mupdf_page_to_pixmap(fz_context *ctx, fz_page *page, fz_matrix ctm, fz_colorspace *cs, float alpha, bool show_extras, mupdf_error_t **errptr)
{
    fz_pixmap *pixmap = NULL;
    fz_try(ctx)
    {
        if (show_extras)
        {
            pixmap = fz_new_pixmap_from_page(ctx, page, ctm, cs, alpha);
        }
        else
        {
            pixmap = fz_new_pixmap_from_page_contents(ctx, page, ctm, cs, alpha);
        }
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return pixmap;
}

/* Cookie */
fz_cookie *mupdf_new_cookie(fz_context *ctx, mupdf_error_t **errptr)
{
    fz_cookie *cookie = NULL;
    fz_try(ctx)
    {
        cookie = fz_malloc_struct(ctx, fz_cookie);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return cookie;
}

/* DisplayList */
fz_display_list *mupdf_new_display_list(fz_context *ctx, fz_rect mediabox, mupdf_error_t **errptr)
{
    fz_display_list *list = NULL;
    fz_try(ctx)
    {
        list = fz_new_display_list(ctx, mediabox);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return list;
}

fz_pixmap *mupdf_display_list_to_pixmap(fz_context *ctx, fz_display_list *list, fz_matrix ctm, fz_colorspace *cs, bool alpha, mupdf_error_t **errptr)
{
    fz_pixmap *pixmap = NULL;
    fz_try(ctx)
    {
        pixmap = fz_new_pixmap_from_display_list(ctx, list, ctm, cs, alpha);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return pixmap;
}

/* PDFObject */
bool mupdf_pdf_is_indirect(fz_context *ctx, pdf_obj *obj, mupdf_error_t **errptr)
{
    int b = 0;
    fz_try(ctx)
    {
        b = pdf_is_indirect(ctx, obj);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return b ? true : false;
}

bool mupdf_pdf_is_null(fz_context *ctx, pdf_obj *obj, mupdf_error_t **errptr)
{
    int b = 0;
    fz_try(ctx)
    {
        b = pdf_is_null(ctx, obj);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return b ? true : false;
}

bool mupdf_pdf_is_bool(fz_context *ctx, pdf_obj *obj, mupdf_error_t **errptr)
{
    int b = 0;
    fz_try(ctx)
    {
        b = pdf_is_bool(ctx, obj);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return b ? true : false;
}

bool mupdf_pdf_is_int(fz_context *ctx, pdf_obj *obj, mupdf_error_t **errptr)
{
    int b = 0;
    fz_try(ctx)
    {
        b = pdf_is_int(ctx, obj);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return b ? true : false;
}

bool mupdf_pdf_is_real(fz_context *ctx, pdf_obj *obj, mupdf_error_t **errptr)
{
    int b = 0;
    fz_try(ctx)
    {
        b = pdf_is_real(ctx, obj);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return b ? true : false;
}

bool mupdf_pdf_is_number(fz_context *ctx, pdf_obj *obj, mupdf_error_t **errptr)
{
    int b = 0;
    fz_try(ctx)
    {
        b = pdf_is_number(ctx, obj);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return b ? true : false;
}

bool mupdf_pdf_is_string(fz_context *ctx, pdf_obj *obj, mupdf_error_t **errptr)
{
    int b = 0;
    fz_try(ctx)
    {
        b = pdf_is_string(ctx, obj);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return b ? true : false;
}

bool mupdf_pdf_is_name(fz_context *ctx, pdf_obj *obj, mupdf_error_t **errptr)
{
    int b = 0;
    fz_try(ctx)
    {
        b = pdf_is_name(ctx, obj);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return b ? true : false;
}

bool mupdf_pdf_is_array(fz_context *ctx, pdf_obj *obj, mupdf_error_t **errptr)
{
    int b = 0;
    fz_try(ctx)
    {
        b = pdf_is_array(ctx, obj);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return b ? true : false;
}

bool mupdf_pdf_is_dict(fz_context *ctx, pdf_obj *obj, mupdf_error_t **errptr)
{
    int b = 0;
    fz_try(ctx)
    {
        b = pdf_is_dict(ctx, obj);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return b ? true : false;
}

bool mupdf_pdf_is_stream(fz_context *ctx, pdf_obj *obj, mupdf_error_t **errptr)
{
    int b = 0;
    fz_try(ctx)
    {
        b = pdf_is_stream(ctx, obj);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return b ? true : false;
}

pdf_obj *mupdf_pdf_new_null()
{
    return PDF_NULL;
}

pdf_obj *mupdf_pdf_new_bool(bool b)
{
    return b ? PDF_TRUE : PDF_FALSE;
}

pdf_obj *mupdf_pdf_new_int(fz_context *ctx, int i, mupdf_error_t **errptr)
{
    pdf_obj *obj = NULL;
    fz_try(ctx)
    {
        obj = pdf_new_int(ctx, i);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return obj;
}

pdf_obj *mupdf_pdf_new_real(fz_context *ctx, float f, mupdf_error_t **errptr)
{
    pdf_obj *obj = NULL;
    fz_try(ctx)
    {
        obj = pdf_new_real(ctx, f);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return obj;
}

pdf_obj *mupdf_pdf_new_string(fz_context *ctx, const char *s, mupdf_error_t **errptr)
{
    pdf_obj *obj = NULL;
    fz_try(ctx)
    {
        obj = pdf_new_text_string(ctx, s);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return obj;
}

pdf_obj *mupdf_pdf_new_name(fz_context *ctx, const char *name, mupdf_error_t **errptr)
{
    pdf_obj *obj = NULL;
    fz_try(ctx)
    {
        obj = pdf_new_name(ctx, name);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return obj;
}

pdf_obj *mupdf_pdf_new_indirect(fz_context *ctx, pdf_document *pdf, int num, int gen, mupdf_error_t **errptr)
{
    pdf_obj *obj = NULL;
    fz_try(ctx)
    {
        obj = pdf_new_indirect(ctx, pdf, num, gen);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return obj;
}

pdf_obj *mupdf_pdf_new_array(fz_context *ctx, pdf_document *pdf, int capacity, mupdf_error_t **errptr)
{
    pdf_obj *obj = NULL;
    fz_try(ctx)
    {
        obj = pdf_new_array(ctx, pdf, capacity);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return obj;
}

pdf_obj *mupdf_pdf_new_dict(fz_context *ctx, pdf_document *pdf, int capacity, mupdf_error_t **errptr)
{
    pdf_obj *obj = NULL;
    fz_try(ctx)
    {
        obj = pdf_new_dict(ctx, pdf, capacity);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return obj;
}

bool mupdf_pdf_to_bool(fz_context *ctx, pdf_obj *obj, mupdf_error_t **errptr)
{
    bool b = false;
    fz_try(ctx)
    {
        b = pdf_to_bool(ctx, obj);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return b;
}

int mupdf_pdf_to_int(fz_context *ctx, pdf_obj *obj, mupdf_error_t **errptr)
{
    int i = 0;
    fz_try(ctx)
    {
        i = pdf_to_int(ctx, obj);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return i;
}

float mupdf_pdf_to_float(fz_context *ctx, pdf_obj *obj, mupdf_error_t **errptr)
{
    float f = 0.0;
    fz_try(ctx)
    {
        f = pdf_to_real(ctx, obj);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return f;
}

int mupdf_pdf_to_indirect(fz_context *ctx, pdf_obj *obj, mupdf_error_t **errptr)
{
    int ind = 0;
    fz_try(ctx)
    {
        ind = pdf_to_num(ctx, obj);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return ind;
}

char *mupdf_pdf_to_string(fz_context *ctx, pdf_obj *obj, mupdf_error_t **errptr)
{
    const char *s = NULL;
    fz_try(ctx)
    {
        s = pdf_to_text_string(ctx, obj);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return strdup(s);
}

char *mupdf_pdf_to_name(fz_context *ctx, pdf_obj *obj, mupdf_error_t **errptr)
{
    const char *s = NULL;
    fz_try(ctx)
    {
        s = pdf_to_name(ctx, obj);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return strdup(s);
}

const unsigned char *mupdf_pdf_to_bytes(fz_context *ctx, pdf_obj *obj, size_t *len, mupdf_error_t **errptr)
{
    const char *s = NULL;
    fz_try(ctx)
    {
        s = pdf_to_string(ctx, obj, len);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return (unsigned char *)s;
}

pdf_obj *mupdf_pdf_resolve_indirect(fz_context *ctx, pdf_obj *obj, mupdf_error_t **errptr)
{
    pdf_obj *ind = NULL;
    fz_try(ctx)
    {
        ind = pdf_resolve_indirect(ctx, obj);
        pdf_keep_obj(ctx, ind);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return ind;
}

pdf_obj *mupdf_pdf_array_get(fz_context *ctx, pdf_obj *obj, int index, mupdf_error_t **errptr)
{
    pdf_obj *val = NULL;
    fz_try(ctx)
    {
        val = pdf_array_get(ctx, obj, index);
        pdf_keep_obj(ctx, val);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return val;
}

pdf_obj *mupdf_pdf_dict_gets(fz_context *ctx, pdf_obj *obj, const char *key, mupdf_error_t **errptr)
{
    pdf_obj *val = NULL;
    fz_try(ctx)
    {
        val = pdf_dict_gets(ctx, obj, key);
        pdf_keep_obj(ctx, val);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return val;
}

pdf_obj *mupdf_pdf_dict_get_inheritable(fz_context *ctx, pdf_obj *obj, pdf_obj *key, mupdf_error_t **errptr)
{
    pdf_obj *val = NULL;
    fz_try(ctx)
    {
        val = pdf_dict_get_inheritable(ctx, obj, key);
        pdf_keep_obj(ctx, val);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return val;
}

fz_buffer *mupdf_pdf_read_stream(fz_context *ctx, pdf_obj *obj, mupdf_error_t **errptr)
{
    fz_buffer *buf = NULL;
    fz_try(ctx)
    {
        buf = pdf_load_stream(ctx, obj);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return buf;
}

fz_buffer *mupdf_pdf_read_raw_stream(fz_context *ctx, pdf_obj *obj, mupdf_error_t **errptr)
{
    fz_buffer *buf = NULL;
    fz_try(ctx)
    {
        buf = pdf_load_raw_stream(ctx, obj);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return buf;
}

void mupdf_pdf_write_object(fz_context *ctx, pdf_obj *self, pdf_obj *obj, mupdf_error_t **errptr)
{
    pdf_document *pdf = pdf_get_bound_document(ctx, self);
    if (!pdf)
    {
        *errptr = mupdf_new_error_from_str("object not bound to document");
        return;
    }
    fz_try(ctx)
    {
        pdf_update_object(ctx, pdf, pdf_to_num(ctx, self), obj);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
}

void mupdf_pdf_write_stream_buffer(fz_context *ctx, pdf_obj *obj, fz_buffer *buf, int compressed, mupdf_error_t **errptr)
{
    pdf_document *pdf = pdf_get_bound_document(ctx, obj);
    if (!pdf)
    {
        *errptr = mupdf_new_error_from_str("object not bound to document");
        return;
    }
    fz_try(ctx)
    {
        pdf_update_stream(ctx, pdf, obj, buf, compressed);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
}

int mupdf_pdf_array_len(fz_context *ctx, pdf_obj *obj, mupdf_error_t **errptr)
{
    int len = 0;
    fz_try(ctx)
    {
        len = pdf_array_len(ctx, obj);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return len;
}

/* Buffer */
size_t mupdf_buffer_read_bytes(fz_context *ctx, fz_buffer *buf, size_t at, unsigned char *output, size_t buf_len, mupdf_error_t **errptr)
{
    size_t remaining_input = 0;
    unsigned char *data;
    size_t len = fz_buffer_storage(ctx, buf, &data);
    if (at == len)
    {
        // EOF
        return 0;
    }
    else if (at > len)
    {
        *errptr = mupdf_new_error_from_str("invalid offset, offset > buffer length");
        return 0;
    }
    remaining_input = len - at;
    len = fz_minz(buf_len, remaining_input);
    memcpy(output, &data[at], len);
    return len;
}

void mupdf_buffer_write_bytes(fz_context *ctx, fz_buffer *buf, const unsigned char *bytes, size_t len, mupdf_error_t **errptr)
{
    fz_try(ctx)
    {
        fz_append_data(ctx, buf, bytes, len);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
}

fz_buffer *mupdf_buffer_from_str(fz_context *ctx, const char *s, mupdf_error_t **errptr)
{
    fz_buffer *buf = NULL;
    fz_try(ctx)
    {
        buf = fz_new_buffer_from_copied_data(ctx, (const unsigned char *)s, strlen(s));
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return buf;
}

fz_buffer *mupdf_buffer_from_base64(fz_context *ctx, const char *s, mupdf_error_t **errptr)
{
    fz_buffer *buf = NULL;
    fz_try(ctx)
    {
        buf = fz_new_buffer_from_base64(ctx, s, strlen(s));
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return buf;
}

/* Document */
fz_document *mupdf_open_document(fz_context *ctx, const char *filename, mupdf_error_t **errptr)
{
    fz_document *doc = NULL;
    fz_try(ctx)
    {
        doc = fz_open_document(ctx, filename);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return doc;
}

fz_document *mupdf_open_document_from_bytes(fz_context *ctx, fz_buffer *bytes, const char *magic, mupdf_error_t **errptr)
{
    if (!magic)
    {
        return NULL;
    }
    fz_document *doc = NULL;
    fz_stream *stream = NULL;
    fz_try(ctx)
    {
        stream = fz_open_buffer(ctx, bytes);
        doc = fz_open_document_with_stream(ctx, magic, stream);
    }
    fz_always(ctx)
    {
        fz_drop_stream(ctx, stream);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return doc;
}

bool mupdf_recognize_document(fz_context *ctx, const char *magic, mupdf_error_t **errptr)
{
    if (!magic)
    {
        return false;
    }
    bool recognized = false;
    fz_try(ctx)
    {
        recognized = fz_recognize_document(ctx, magic) != NULL;
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return recognized;
}

bool mupdf_needs_password(fz_context *ctx, fz_document *doc, mupdf_error_t **errptr)
{
    bool needs = false;
    fz_try(ctx)
    {
        needs = fz_needs_password(ctx, doc);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return needs;
}

bool mupdf_authenticate_password(fz_context *ctx, fz_document *doc, const char *password, mupdf_error_t **errptr)
{
    bool ok = false;
    fz_try(ctx)
    {
        ok = fz_authenticate_password(ctx, doc, password);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return ok;
}

int mupdf_document_page_count(fz_context *ctx, fz_document *doc, mupdf_error_t **errptr)
{
    int count = 0;
    fz_try(ctx)
    {
        count = fz_count_pages(ctx, doc);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return count;
}

char *mupdf_lookup_metadata(fz_context *ctx, fz_document *doc, const char *key, char info[], int info_len, mupdf_error_t **errptr)
{
    fz_try(ctx)
    {
        fz_lookup_metadata(ctx, doc, key, info, info_len);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return info;
}

bool mupdf_is_document_reflowable(fz_context *ctx, fz_document *doc, mupdf_error_t **errptr)
{
    bool is_reflowable = false;
    fz_try(ctx)
    {
        is_reflowable = fz_is_document_reflowable(ctx, doc);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return is_reflowable;
}

void mupdf_layout_document(fz_context *ctx, fz_document *doc, float w, float h, float em, mupdf_error_t **errptr)
{
    fz_try(ctx)
    {
        fz_layout_document(ctx, doc, w, h, em);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
}

fz_page *mupdf_load_page(fz_context *ctx, fz_document *doc, int page_no, mupdf_error_t **errptr)
{
    fz_page *page = NULL;
    fz_try(ctx)
    {
        page = fz_load_page(ctx, doc, page_no);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return page;
}

pdf_obj *mupdf_pdf_add_object(fz_context *ctx, pdf_document *pdf, pdf_obj *obj, mupdf_error_t **errptr)
{
    pdf_obj *ind = NULL;
    fz_try(ctx)
    {
        ind = pdf_add_object(ctx, pdf, obj);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return ind;
}

pdf_obj *mupdf_pdf_create_object(fz_context *ctx, pdf_document *pdf, mupdf_error_t **errptr)
{
    pdf_obj *ind = NULL;
    fz_try(ctx)
    {
        ind = pdf_new_indirect(ctx, pdf, pdf_create_object(ctx, pdf), 0);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return ind;
}

void mupdf_pdf_delete_object(fz_context *ctx, pdf_document *pdf, int num, mupdf_error_t **errptr)
{
    fz_try(ctx)
    {
        pdf_delete_object(ctx, pdf, num);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
}

pdf_obj *mupdf_pdf_add_image(fz_context *ctx, pdf_document *pdf, fz_image *image, mupdf_error_t **errptr)
{
    pdf_obj *ind = NULL;
    fz_try(ctx)
    {
        ind = pdf_add_image(ctx, pdf, image);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return ind;
}

pdf_obj *mupdf_pdf_add_font(fz_context *ctx, pdf_document *pdf, fz_font *font, mupdf_error_t **errptr)
{
    pdf_obj *ind = NULL;
    fz_try(ctx)
    {
        ind = pdf_add_cid_font(ctx, pdf, font);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return ind;
}

pdf_obj *mupdf_pdf_add_cjk_font(fz_context *ctx, pdf_document *pdf, fz_font *font, int ordering, int wmode, bool serif, mupdf_error_t **errptr)
{
    pdf_obj *ind = NULL;
    fz_try(ctx)
    {
        ind = pdf_add_cjk_font(ctx, pdf, font, ordering, wmode, serif);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return ind;
}

pdf_obj *mupdf_pdf_add_simple_font(fz_context *ctx, pdf_document *pdf, fz_font *font, int encoding, mupdf_error_t **errptr)
{
    pdf_obj *ind = NULL;
    fz_try(ctx)
    {
        ind = pdf_add_simple_font(ctx, pdf, font, encoding);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return ind;
}

void mupdf_pdf_save_document(fz_context *ctx, pdf_document *pdf, const char *filename, pdf_write_options pwo, mupdf_error_t **errptr)
{
    fz_try(ctx)
    {
        pdf_save_document(ctx, pdf, filename, &pwo);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
}

fz_buffer *mupdf_pdf_write_document(fz_context *ctx, pdf_document *pdf, pdf_write_options pwo, mupdf_error_t **errptr)
{
    fz_output *out = NULL;
    fz_buffer *buf = NULL;
    fz_var(out);
    fz_try(ctx)
    {
        buf = fz_new_buffer(ctx, 8192);
        out = fz_new_output_with_buffer(ctx, buf);
        pdf_write_document(ctx, pdf, out, &pwo);
        fz_close_output(ctx, out);
    }
    fz_always(ctx)
    {
        fz_drop_output(ctx, out);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return buf;
}

void mupdf_pdf_enable_js(fz_context *ctx, pdf_document *pdf, mupdf_error_t **errptr)
{
    fz_try(ctx)
    {
        pdf_enable_js(ctx, pdf);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
}

void mupdf_pdf_disable_js(fz_context *ctx, pdf_document *pdf, mupdf_error_t **errptr)
{
    fz_try(ctx)
    {
        pdf_disable_js(ctx, pdf);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
}

bool mupdf_pdf_js_supported(fz_context *ctx, pdf_document *pdf, mupdf_error_t **errptr)
{
    bool supported = false;
    fz_try(ctx)
    {
        supported = pdf_js_supported(ctx, pdf);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return supported;
}

void mupdf_pdf_calculate_form(fz_context *ctx, pdf_document *pdf, mupdf_error_t **errptr)
{
    fz_try(ctx)
    {
        if (pdf->recalculate)
        {
            pdf_calculate_form(ctx, pdf);
        }
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
}

pdf_obj *mupdf_pdf_trailer(fz_context *ctx, pdf_document *pdf, mupdf_error_t **errptr)
{
    pdf_obj *obj = NULL;
    fz_try(ctx)
    {
        obj = pdf_trailer(ctx, pdf);
        pdf_keep_obj(ctx, obj);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return obj;
}

int mupdf_pdf_count_objects(fz_context *ctx, pdf_document *pdf, mupdf_error_t **errptr)
{
    int count = 0;
    fz_try(ctx)
    {
        count = pdf_xref_len(ctx, pdf);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return count;
}

pdf_graft_map *mupdf_pdf_new_graft_map(fz_context *ctx, pdf_document *pdf, mupdf_error_t **errptr)
{
    pdf_graft_map *map = NULL;
    fz_try(ctx)
    {
        map = pdf_new_graft_map(ctx, pdf);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return map;
}

pdf_obj *mupdf_pdf_graft_object(fz_context *ctx, pdf_document *doc, pdf_obj *obj, mupdf_error_t **errptr)
{
    pdf_obj *graft_obj = NULL;
    fz_try(ctx)
    {
        graft_obj = pdf_graft_object(ctx, doc, obj);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return graft_obj;
}

pdf_page *mupdf_pdf_new_page(fz_context *ctx, pdf_document *pdf, int page_no, float width, float height, mupdf_error_t **errptr)
{
    fz_rect mediabox = fz_unit_rect;
    mediabox.x1 = width;
    mediabox.y1 = height;
    pdf_obj *resources = NULL, *page_obj = NULL;
    fz_buffer *contents = NULL;
    pdf_page *page = NULL;
    fz_try(ctx)
    {
        // create /Resources and /Contents objects
        resources = pdf_add_object_drop(ctx, pdf, pdf_new_dict(ctx, pdf, 1));
        page_obj = pdf_add_page(ctx, pdf, mediabox, 0, resources, contents);
        pdf_insert_page(ctx, pdf, page_no, page_obj);
        pdf->dirty = 1;
        int n = page_no;
        int page_count = pdf_count_pages(ctx, pdf);
        while (n < 0) {
            n += page_count;
        }
        fz_page *fz_page = fz_load_page(ctx, &pdf->super, n);
        page = pdf_page_from_fz_page(ctx, fz_page);
    }
    fz_always(ctx)
    {
        fz_drop_buffer(ctx, contents);
        pdf_drop_obj(ctx, page_obj);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return page;
}

pdf_obj *mupdf_pdf_lookup_page_obj(fz_context *ctx, pdf_document *pdf, int page_no, mupdf_error_t **errptr)
{
    pdf_obj *obj = NULL;
    fz_try(ctx)
    {
        obj =pdf_lookup_page_obj(ctx, pdf, page_no);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return obj;
}

void mupdf_pdf_insert_page(fz_context *ctx, pdf_document *pdf, int page_no, pdf_obj *page, mupdf_error_t **errptr)
{
    if (page_no < 0 || page_no >= pdf_count_pages(ctx, pdf)) {
        *errptr = mupdf_new_error_from_str("page_no is not a valid page");
        return;
    }
    fz_try(ctx)
    {
        pdf_insert_page(ctx, pdf, page_no, page);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
}

void mupdf_pdf_delete_page(fz_context *ctx, pdf_document *pdf, int page_no, mupdf_error_t **errptr)
{
    if (page_no < 0 || page_no >= pdf_count_pages(ctx, pdf)) {
        *errptr = mupdf_new_error_from_str("page_no is not a valid page");
        return;
    }
    fz_try(ctx)
    {
        pdf_delete_page(ctx, pdf, page_no);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
}

/* DrawDevice */
fz_device *mupdf_new_draw_device(fz_context *ctx, fz_pixmap *pixmap, mupdf_error_t **errptr)
{
    fz_device *device = NULL;
    fz_try(ctx)
    {
        device = fz_new_draw_device(ctx, fz_identity, pixmap);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return device;
}

/* DisplayListDevice */
fz_device *mupdf_new_display_list_device(fz_context *ctx, fz_display_list *list, mupdf_error_t **errptr)
{
    fz_device *device = NULL;
    fz_try(ctx)
    {
        device = fz_new_list_device(ctx, list);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return device;
}

/* PdfPage */
pdf_annot *mupdf_pdf_create_annot(fz_context *ctx, pdf_page *page, int subtype, mupdf_error_t **errptr)
{
    pdf_annot *annot = NULL;
    fz_try(ctx)
    {
        annot = pdf_create_annot(ctx, page, subtype);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return annot;
}

void mupdf_pdf_delete_annot(fz_context *ctx, pdf_page *page, pdf_annot *annot, mupdf_error_t **errptr)
{
    fz_try(ctx)
    {
        pdf_delete_annot(ctx, page, annot);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
}

bool mupdf_pdf_update_page(fz_context *ctx, pdf_page *page, mupdf_error_t **errptr)
{
    bool updated = false;
    fz_try(ctx)
    {
        updated = pdf_update_page(ctx, page);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return updated;
}

bool mupdf_pdf_redact_page(fz_context *ctx, pdf_page *page, mupdf_error_t **errptr)
{
    bool redacted = false;
    fz_try(ctx)
    {
        redacted = pdf_redact_page(ctx, page->doc, page, NULL);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return redacted;
}

/* PDFAnnotation */
int mupdf_pdf_annot_type(fz_context *ctx, pdf_annot *annot, mupdf_error_t **errptr)
{
    int subtype = 0;
    fz_try(ctx)
    {
        subtype = pdf_annot_type(ctx, annot);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return subtype;
}
