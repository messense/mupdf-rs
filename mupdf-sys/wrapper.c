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
	unlock
};

typedef struct mupdf_error
{
    int type;
    char *message;
} mupdf_error_t;

void mupdf_save_error(fz_context *ctx, mupdf_error_t **errptr)
{
    assert(errptr != NULL);
    int type = fz_caught(ctx);
    const char *message = fz_caught_message(ctx);
    mupdf_error_t *err = malloc(sizeof(mupdf_error_t));
    err->type = type;
    err->message = strdup(message);
    *errptr = err;
}

void mupdf_drop_error(mupdf_error_t *err) {
    if (err == NULL) {
        return;
    }
    if (err->message != NULL) {
        free(err->message);
    }
    free(err);
    err = NULL;
}

/* Context */
fz_context *mupdf_new_base_context()
{
    for (int i = 0; i < FZ_LOCK_MAX; i++) {
#ifdef _WIN32
		InitializeCriticalSection(&mutexes[i]);
#else
		(void)pthread_mutex_init(&mutexes[i], NULL);
#endif
    }
    fz_context *ctx = fz_new_context(NULL, &locks, FZ_STORE_DEFAULT);
    if (!ctx) {
        return NULL;
    }
    fz_register_document_handlers(ctx);
    return ctx;
}

void mupdf_drop_base_context(fz_context *ctx)
{
    for (int i = 0; i < FZ_LOCK_MAX; i++) {
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
        if (data) {
            font = fz_new_font_from_memory(ctx, name, data, size, index, 0);
        } else {
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
        if (show_extras) {
            pixmap = fz_new_pixmap_from_page(ctx, page, ctm, cs, alpha);
        } else {
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

pdf_obj *mupdf_pdf_new_null() {
    return PDF_NULL;
}

pdf_obj *mupdf_pdf_new_bool(bool b) {
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

/* Buffer */
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
    if (!magic) {
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
    if (!magic) {
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

char* mupdf_lookup_metadata(fz_context *ctx, fz_document *doc, const char *key, char info[], int info_len, mupdf_error_t **errptr)
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

pdf_obj *mupdf_document_add_object(fz_context *ctx, pdf_document *pdf, pdf_obj *obj, mupdf_error_t **errptr)
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

pdf_obj *mupdf_document_create_object(fz_context *ctx, pdf_document *pdf, mupdf_error_t **errptr)
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

void mupdf_document_delete_object(fz_context *ctx, pdf_document *pdf, int num, mupdf_error_t **errptr)
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
