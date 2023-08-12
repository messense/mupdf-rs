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

#ifdef HAVE_ANDROID
#include "androidfonts.c"
#endif

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

static mupdf_error_t *mupdf_new_error_from_str(const char *message)
{
    mupdf_error_t *err = malloc(sizeof(mupdf_error_t));
    err->type = FZ_ERROR_GENERIC;
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
void mupdf_drop_base_context(fz_context *ctx)
{
    int i;
    for (i = 0; i < FZ_LOCK_MAX; i++)
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

fz_context *mupdf_new_base_context()
{
    int i;
    for (i = 0; i < FZ_LOCK_MAX; i++)
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
        mupdf_drop_base_context(ctx);
        return NULL;
    }
    fz_try(ctx) {
        fz_register_document_handlers(ctx);
    }
    fz_catch(ctx) {
        mupdf_drop_base_context(ctx);
    }
    // Disable default warning & error printing
    fz_set_warning_callback(ctx, NULL, NULL);
    fz_set_error_callback(ctx, NULL, NULL);
#ifdef HAVE_ANDROID
    fz_install_load_system_font_funcs(ctx,
		load_droid_font,
		load_droid_cjk_font,
		load_droid_fallback_font);
#endif
    return ctx;
}

/* Rect */
fz_rect mupdf_adjust_rect_for_stroke(fz_context *ctx, fz_rect self, fz_stroke_state *stroke, fz_matrix ctm, mupdf_error_t **errptr)
{
    fz_rect rect;
    fz_try(ctx)
    {
        rect = fz_adjust_rect_for_stroke(ctx, self, stroke, ctm);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return rect;
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

fz_pixmap *mupdf_clone_pixmap(fz_context *ctx, fz_pixmap *self, mupdf_error_t **errptr)
{
    fz_pixmap *pixmap = NULL;
    fz_try(ctx)
    {
        pixmap = fz_clone_pixmap(ctx, self);
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
    if (!fz_pixmap_colorspace(ctx, pixmap))
    {
        *errptr = mupdf_new_error_from_str("colorspace invalid for function");
        return;
    }
    fz_try(ctx)
    {
        fz_gamma_pixmap(ctx, pixmap, gamma);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
}

void mupdf_tint_pixmap(fz_context *ctx, fz_pixmap *pixmap, int black, int white, mupdf_error_t **errptr)
{
    fz_colorspace *cs = fz_pixmap_colorspace(ctx, pixmap);
    if (!cs || cs->n > 3)
    {
        *errptr = mupdf_new_error_from_str("colorspace invalid for function");
        return;
    }
    fz_try(ctx)
    {
        fz_tint_pixmap(ctx, pixmap, black, white);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
}

void mupdf_save_pixmap_as(fz_context *ctx, fz_pixmap *pixmap, const char *filename, int format, mupdf_error_t **errptr)
{
    fz_try(ctx)
    {
        switch (format)
        {
        case (0):
            fz_save_pixmap_as_png(ctx, pixmap, filename);
            break;
        case (1):
            fz_save_pixmap_as_pnm(ctx, pixmap, filename);
            break;
        case (2):
            fz_save_pixmap_as_pam(ctx, pixmap, filename);
            break;
        case (3): // Adobe Photoshop Document
            fz_save_pixmap_as_psd(ctx, pixmap, filename);
            break;
        case (4): // Postscript format
            fz_save_pixmap_as_ps(ctx, pixmap, (char *)filename, 0);
            break;
        default:
            fz_save_pixmap_as_png(ctx, pixmap, filename);
            break;
        }
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
}

fz_buffer *mupdf_pixmap_get_image_data(fz_context *ctx, fz_pixmap *pixmap, int format, mupdf_error_t **errptr)
{
    fz_output *out = NULL;
    fz_buffer *buf = NULL;
    fz_var(out);
    fz_var(buf);
    fz_try(ctx)
    {
        size_t size = fz_pixmap_stride(ctx, pixmap) * pixmap->h;
        buf = fz_new_buffer(ctx, size);
        out = fz_new_output_with_buffer(ctx, buf);
        switch (format)
        {
        case (0):
            fz_write_pixmap_as_png(ctx, out, pixmap);
            break;
        case (1):
            fz_write_pixmap_as_pnm(ctx, out, pixmap);
            break;
        case (2):
            fz_write_pixmap_as_pam(ctx, out, pixmap);
            break;
        case (3): // Adobe Photoshop Document
            fz_write_pixmap_as_psd(ctx, out, pixmap);
            break;
        case (4): // Postscript format
            fz_write_pixmap_as_ps(ctx, out, pixmap);
            break;
        default:
            fz_write_pixmap_as_png(ctx, out, pixmap);
            break;
        }
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

fz_font *mupdf_new_font_from_buffer(fz_context *ctx, const char *name, int index, fz_buffer *buffer, mupdf_error_t **errptr)
{
    fz_font *font = NULL;
    fz_try(ctx)
    {
        font = fz_new_font_from_buffer(ctx, name, buffer, index, 0);
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

fz_path *mupdf_outline_glyph(fz_context *ctx, fz_font *font, int glyph, fz_matrix ctm, mupdf_error_t **errptr)
{
    fz_path *path = NULL;
    fz_try(ctx)
    {
        path = fz_outline_glyph(ctx, font, glyph, ctm);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return path;
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

fz_image *mupdf_new_image_from_display_list(fz_context *ctx, fz_display_list *list, float w, float h, mupdf_error_t **errptr)
{
    fz_image *image = NULL;
    fz_try(ctx)
    {
        image = fz_new_image_from_display_list(ctx, w, h, list);
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
fz_stroke_state *mupdf_default_stroke_state(fz_context *ctx)
{
    fz_stroke_state *stroke = NULL;
    stroke = fz_clone_stroke_state(ctx, (fz_stroke_state *)&fz_default_stroke_state);
    return stroke;
}

fz_stroke_state *mupdf_new_stroke_state(
    fz_context *ctx, fz_linecap start_cap, fz_linecap dash_cap, fz_linecap end_cap, fz_linejoin line_join, float line_width,
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

void mupdf_trim_path(fz_context *ctx, fz_path *path, mupdf_error_t **errptr)
{
    fz_try(ctx)
    {
        fz_trim_path(ctx, path);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
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

void mupdf_walk_path(fz_context *ctx, const fz_path *path, const fz_path_walker *walker, void *arg, mupdf_error_t **errptr)
{
    fz_try(ctx)
    {
        fz_walk_path(ctx, path, walker, arg);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
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

fz_buffer *mupdf_page_to_svg(fz_context *ctx, fz_page *page, fz_matrix ctm, fz_cookie *cookie, mupdf_error_t **errptr)
{
    fz_rect mediabox = fz_bound_page(ctx, page);
    fz_device *dev = NULL;
    fz_buffer *buf = NULL;
    fz_output *out = NULL;
    fz_var(out);
    fz_var(dev);
    fz_var(buf);
    fz_rect tbounds = mediabox;
    tbounds = fz_transform_rect(tbounds, ctm);
    fz_try(ctx)
    {
        buf = fz_new_buffer(ctx, 1024);
        out = fz_new_output_with_buffer(ctx, buf);
        dev = fz_new_svg_device(ctx, out, tbounds.x1 - tbounds.x0, tbounds.y1 - tbounds.y0, FZ_SVG_TEXT_AS_PATH, 1);
        fz_run_page(ctx, page, dev, ctm, cookie);
        fz_close_device(ctx, dev);
    }
    fz_always(ctx)
    {
        fz_drop_device(ctx, dev);
        fz_drop_output(ctx, out);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return buf;
}

fz_stext_page *mupdf_page_to_text_page(fz_context *ctx, fz_page *page, int flags, mupdf_error_t **errptr)
{
    fz_stext_page *text_page = NULL;
    fz_stext_options opts = {0};
    opts.flags = flags;
    fz_try(ctx)
    {
        text_page = fz_new_stext_page_from_page(ctx, page, &opts);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return text_page;
}

fz_display_list *mupdf_page_to_display_list(fz_context *ctx, fz_page *page, bool annots, mupdf_error_t **errptr)
{
    fz_display_list *list = NULL;
    fz_try(ctx)
    {
        if (annots)
        {
            list = fz_new_display_list_from_page(ctx, page);
        }
        else
        {
            list = fz_new_display_list_from_page_contents(ctx, page);
        }
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return list;
}

void mupdf_run_page(fz_context *ctx, fz_page *page, fz_device *device, fz_matrix ctm, fz_cookie *cookie, mupdf_error_t **errptr)
{
    fz_try(ctx)
    {
        fz_run_page(ctx, page, device, ctm, cookie);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
}

void mupdf_run_page_contents(fz_context *ctx, fz_page *page, fz_device *device, fz_matrix ctm, fz_cookie *cookie, mupdf_error_t **errptr)
{
    fz_try(ctx)
    {
        fz_run_page_contents(ctx, page, device, ctm, cookie);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
}

void mupdf_run_page_annots(fz_context *ctx, fz_page *page, fz_device *device, fz_matrix ctm, fz_cookie *cookie, mupdf_error_t **errptr)
{
    fz_try(ctx)
    {
        fz_run_page_annots(ctx, page, device, ctm, cookie);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
}

void mupdf_run_page_widgets(fz_context *ctx, fz_page *page, fz_device *device, fz_matrix ctm, fz_cookie *cookie, mupdf_error_t **errptr)
{
    fz_try(ctx)
    {
        fz_run_page_widgets(ctx, page, device, ctm, cookie);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
}

fz_buffer *mupdf_page_to_html(fz_context *ctx, fz_page *page, mupdf_error_t **errptr)
{
    fz_buffer *buf = NULL;
    fz_output *out = NULL;
    fz_stext_page *text = NULL;
    fz_var(text);
    fz_var(buf);
    fz_var(out);
    fz_try(ctx)
    {
        text = fz_new_stext_page_from_page(ctx, page, NULL);
        buf = fz_new_buffer(ctx, 8192);
        out = fz_new_output_with_buffer(ctx, buf);
        fz_print_stext_header_as_html(ctx, out);
        fz_print_stext_page_as_html(ctx, out, text, page->number);
        fz_print_stext_trailer_as_html(ctx, out);
        fz_close_output(ctx, out);
    }
    fz_always(ctx)
    {
        fz_drop_output(ctx, out);
        fz_drop_stext_page(ctx, text);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return buf;
}

fz_buffer *mupdf_page_to_xhtml(fz_context *ctx, fz_page *page, mupdf_error_t **errptr)
{
    fz_buffer *buf = NULL;
    fz_output *out = NULL;
    fz_stext_page *text = NULL;
    fz_var(text);
    fz_var(buf);
    fz_var(out);
    fz_try(ctx)
    {
        text = fz_new_stext_page_from_page(ctx, page, NULL);
        buf = fz_new_buffer(ctx, 8192);
        out = fz_new_output_with_buffer(ctx, buf);
        fz_print_stext_header_as_xhtml(ctx, out);
        fz_print_stext_page_as_xhtml(ctx, out, text, page->number);
        fz_print_stext_trailer_as_xhtml(ctx, out);
        fz_close_output(ctx, out);
    }
    fz_always(ctx)
    {
        fz_drop_output(ctx, out);
        fz_drop_stext_page(ctx, text);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return buf;
}

fz_buffer *mupdf_page_to_xml(fz_context *ctx, fz_page *page, mupdf_error_t **errptr)
{
    fz_buffer *buf = NULL;
    fz_output *out = NULL;
    fz_stext_page *text = NULL;
    fz_var(text);
    fz_var(buf);
    fz_var(out);
    fz_try(ctx)
    {
        text = fz_new_stext_page_from_page(ctx, page, NULL);
        buf = fz_new_buffer(ctx, 8192);
        out = fz_new_output_with_buffer(ctx, buf);
        fz_print_stext_page_as_xml(ctx, out, text, page->number);
        fz_close_output(ctx, out);
    }
    fz_always(ctx)
    {
        fz_drop_output(ctx, out);
        fz_drop_stext_page(ctx, text);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return buf;
}

fz_buffer *mupdf_page_to_text(fz_context *ctx, fz_page *page, mupdf_error_t **errptr)
{
    fz_buffer *buf = NULL;
    fz_output *out = NULL;
    fz_stext_page *text = NULL;
    fz_var(text);
    fz_var(buf);
    fz_var(out);
    fz_try(ctx)
    {
        text = fz_new_stext_page_from_page(ctx, page, NULL);
        buf = fz_new_buffer(ctx, 8192);
        out = fz_new_output_with_buffer(ctx, buf);
        fz_print_stext_page_as_text(ctx, out, text);
        fz_close_output(ctx, out);
    }
    fz_always(ctx)
    {
        fz_drop_output(ctx, out);
        fz_drop_stext_page(ctx, text);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return buf;
}

fz_link *mupdf_load_links(fz_context *ctx, fz_page *page, mupdf_error_t **errptr)
{
    fz_link *link = NULL;
    fz_try(ctx)
    {
        link = fz_load_links(ctx, page);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return link;
}

fz_buffer *mupdf_stext_page_to_text(fz_context *ctx, fz_stext_page *page, mupdf_error_t **errptr)
{
    fz_buffer *buf = NULL;
    fz_output *out = NULL;
    fz_var(buf);
    fz_var(out);
    fz_try(ctx)
    {
        buf = fz_new_buffer(ctx, 8192);
        out = fz_new_output_with_buffer(ctx, buf);
        fz_print_stext_page_as_text(ctx, out, page);
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

fz_separations *mupdf_page_separations(fz_context *ctx, fz_page *page, mupdf_error_t **errptr)
{
    fz_separations *seps = NULL;
    fz_try(ctx)
    {
        seps = fz_page_separations(ctx, page);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return seps;
}

fz_quad *mupdf_search_page(fz_context *ctx, fz_page *page, const char *needle, const int hit_max, int *hit_count, mupdf_error_t **errptr)
{
    fz_quad *result = NULL;
    fz_var(result);
    fz_try(ctx)
    {
        result = fz_calloc(ctx, hit_max, sizeof(fz_quad));
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    if (result == NULL) {
        return NULL;
    }
    fz_try(ctx)
    {
        *hit_count = fz_search_page(ctx, page, needle, NULL, result, hit_max);
    }
    fz_catch(ctx)
    {
        fz_free(ctx, result);
        mupdf_save_error(ctx, errptr);
    }
    return result;
}

fz_quad *mupdf_search_stext_page(fz_context *ctx, fz_stext_page *page, const char *needle, const int hit_max, int *hit_count, mupdf_error_t **errptr)
{
    fz_quad *result = NULL;
    fz_var(result);
    fz_try(ctx)
    {
        result = fz_calloc(ctx, hit_max, sizeof(fz_quad));
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    if (result == NULL) {
        return NULL;
    }
    fz_try(ctx)
    {
        *hit_count = fz_search_stext_page(ctx, page, needle, NULL, result, hit_max);
    }
    fz_catch(ctx)
    {
        fz_free(ctx, result);
        mupdf_save_error(ctx, errptr);
    }
    return result;
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

fz_stext_page *mupdf_display_list_to_text_page(fz_context *ctx, fz_display_list *list, int flags, mupdf_error_t **errptr)
{
    fz_stext_page *text_page = NULL;
    fz_stext_options opts = {0};
    opts.flags = flags;
    fz_try(ctx)
    {
        text_page = fz_new_stext_page_from_display_list(ctx, list, &opts);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return text_page;
}

void mupdf_display_list_run(fz_context *ctx, fz_display_list *list, fz_device *device, fz_matrix ctm, fz_rect area, fz_cookie *cookie, mupdf_error_t **errptr)
{
    fz_try(ctx)
    {
        fz_run_display_list(ctx, list, device, ctm, area, cookie);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
}

fz_quad *mupdf_search_display_list(fz_context *ctx, fz_display_list *list, const char *needle, const int hit_max, int *hit_count, mupdf_error_t **errptr)
{
    fz_quad *result = NULL;
    fz_var(result);
    fz_try(ctx)
    {
        result = fz_calloc(ctx, hit_max, sizeof(fz_quad));
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    if (result == NULL) {
        return NULL;
    }
    fz_try(ctx)
    {
        *hit_count = fz_search_display_list(ctx, list, needle, NULL, result, hit_max);
    }
    fz_catch(ctx)
    {
        fz_free(ctx, result);
        mupdf_save_error(ctx, errptr);
    }
    return result;
}

/* PDFObject */
pdf_obj *mupdf_pdf_clone_obj(fz_context *ctx, pdf_obj *self, mupdf_error_t **errptr)
{
    pdf_obj *obj = NULL;
    fz_try(ctx)
    {
        obj = pdf_deep_copy_obj(ctx, self);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return obj;
}

pdf_document *mupdf_pdf_get_bound_document(fz_context *ctx, pdf_obj *obj)
{
    pdf_document *pdf = NULL;
    fz_try(ctx)
    {
        pdf = pdf_get_bound_document(ctx, obj);
        pdf_keep_document(ctx, pdf);
    }
    fz_catch(ctx)
    {
    }
    return pdf;
}

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

pdf_obj *mupdf_pdf_obj_from_str(fz_context *ctx, pdf_document *pdf, const char *src, mupdf_error_t **errptr)
{
    pdf_obj *obj = NULL;
    pdf_lexbuf lexbuf;
    fz_stream *stream = fz_open_memory(ctx, (unsigned char *)src, strlen(src));
    pdf_lexbuf_init(ctx, &lexbuf, PDF_LEXBUF_SMALL);
    fz_var(stream);
    fz_try(ctx)
    {
        obj = pdf_parse_stm_obj(ctx, pdf, stream, &lexbuf);
    }
    fz_always(ctx)
    {
        pdf_lexbuf_fin(ctx, &lexbuf);
        fz_drop_stream(ctx, stream);
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

const char *mupdf_pdf_to_string(fz_context *ctx, pdf_obj *obj, mupdf_error_t **errptr)
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
    return s;
}

const char *mupdf_pdf_to_name(fz_context *ctx, pdf_obj *obj, mupdf_error_t **errptr)
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
    return s;
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

pdf_obj *mupdf_pdf_dict_get_val(fz_context *ctx, pdf_obj *obj, int idx, mupdf_error_t **errptr)
{
    pdf_obj *val = NULL;
    fz_try(ctx)
    {
        val = pdf_dict_get_val(ctx, obj, idx);
        pdf_keep_obj(ctx, val);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return val;
}

pdf_obj *mupdf_pdf_dict_get_key(fz_context *ctx, pdf_obj *obj, int idx, mupdf_error_t **errptr)
{
    pdf_obj *val = NULL;
    fz_try(ctx)
    {
        val = pdf_dict_get_key(ctx, obj, idx);
        pdf_keep_obj(ctx, val);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return val;
}

pdf_obj *mupdf_pdf_dict_get(fz_context *ctx, pdf_obj *obj, pdf_obj *key, mupdf_error_t **errptr)
{
    pdf_obj *val = NULL;
    fz_try(ctx)
    {
        val = pdf_dict_get(ctx, obj, key);
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

void mupdf_pdf_array_put(fz_context *ctx, pdf_obj *self, int i, pdf_obj *item, mupdf_error_t **errptr)
{
    fz_try(ctx)
    {
        pdf_array_put(ctx, self, i, item);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
}

void mupdf_pdf_array_push(fz_context *ctx, pdf_obj *self, pdf_obj *item, mupdf_error_t **errptr)
{
    fz_try(ctx)
    {
        pdf_array_push(ctx, self, item);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
}

void mupdf_pdf_array_delete(fz_context *ctx, pdf_obj *self, int i, mupdf_error_t **errptr)
{
    fz_try(ctx)
    {
        pdf_array_delete(ctx, self, i);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
}

int mupdf_pdf_dict_len(fz_context *ctx, pdf_obj *obj, mupdf_error_t **errptr)
{
    int len = 0;
    fz_try(ctx)
    {
        len = pdf_dict_len(ctx, obj);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return len;
}

void mupdf_pdf_dict_put(fz_context *ctx, pdf_obj *self, pdf_obj *key, pdf_obj *value, mupdf_error_t **errptr)
{
    fz_try(ctx)
    {
        pdf_dict_put(ctx, self, key, value);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
}

void mupdf_pdf_dict_delete(fz_context *ctx, pdf_obj *self, pdf_obj *key, mupdf_error_t **errptr)
{
    fz_try(ctx)
    {
        pdf_dict_del(ctx, self, key);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
}

char *mupdf_pdf_obj_to_string(fz_context *ctx, pdf_obj *obj, bool tight, bool ascii, mupdf_error_t **errptr)
{
    char *s = NULL;
    size_t n = 0;
    fz_var(s);
    fz_try(ctx)
    {
        s = pdf_sprint_obj(ctx, NULL, 0, &n, obj, tight, ascii);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return s;
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
    fz_var(stream);
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

char *mupdf_lookup_metadata(fz_context *ctx, fz_document *doc, const char *key, mupdf_error_t **errptr)
{
    int len;
    char *value = NULL;
    fz_try(ctx)
    {
        len = fz_lookup_metadata(ctx, doc, key, NULL, 0) + 1;
        if (len > 1)
        {
            value = calloc(len, sizeof(char));
            fz_lookup_metadata(ctx, doc, key, value, len);
        }
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return value;
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
        if (!fz_is_document_reflowable(ctx, doc))
        {
            return;
        }
        if (w <= 0.0f || h <= 0.0f)
        {
            *errptr = mupdf_new_error_from_str("invalid width or height");
            return;
        }
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

static pdf_document *mupdf_convert_to_pdf_internal(fz_context *ctx, fz_document *doc, int fp, int tp, int rotate, fz_cookie *cookie)
{
    pdf_document *pdfout = pdf_create_document(ctx);
    int i, incr = 1, s = fp, e = tp;
    if (fp > tp) // revert page sequence
    {
        incr = -1;
        s = tp;
        e = fp;
    }
    fz_rect mediabox;
    fz_device *dev = NULL;
    fz_buffer *contents = NULL;
    pdf_obj *resources = NULL;
    fz_page *page;
    fz_var(dev);
    fz_var(contents);
    fz_var(resources);
    fz_var(page);
    for (i = fp; i >= s && i <= e; i += incr)
    {
        fz_try(ctx)
        {
            page = fz_load_page(ctx, doc, i);
            mediabox = fz_bound_page(ctx, page);
            dev = pdf_page_write(ctx, pdfout, mediabox, &resources, &contents);
            fz_run_page(ctx, page, dev, fz_identity, cookie);
            fz_close_device(ctx, dev);
            fz_drop_device(ctx, dev);
            dev = NULL;
            pdf_obj *page_obj = pdf_add_page(ctx, pdfout, mediabox, rotate, resources, contents);
            pdf_insert_page(ctx, pdfout, -1, page_obj);
            pdf_drop_obj(ctx, page_obj);
        }
        fz_always(ctx)
        {
            pdf_drop_obj(ctx, resources);
            fz_drop_buffer(ctx, contents);
            fz_drop_device(ctx, dev);
            fz_drop_page(ctx, page);
        }
        fz_catch(ctx)
        {
            fz_rethrow(ctx);
        }
    }
    return pdfout;
}

pdf_document *mupdf_convert_to_pdf(fz_context *ctx, fz_document *doc, int fp, int tp, int rotate, fz_cookie *cookie, mupdf_error_t **errptr)
{
    if (rotate % 90)
    {
        *errptr = mupdf_new_error_from_str("rotation not multiple of 90");
        return NULL;
    }
    pdf_document *pdf = NULL;
    fz_try(ctx)
    {
        pdf = mupdf_convert_to_pdf_internal(ctx, doc, fp, tp, rotate, cookie);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return pdf;
}

fz_location mupdf_resolve_link(fz_context *ctx, fz_document *doc, const char *uri, mupdf_error_t **errptr)
{
    fz_location loc = { -1, -1 };
    float xp = 0.0f, yp = 0.0f;
    fz_try(ctx)
    {
        loc = fz_resolve_link(ctx, doc, uri, &xp, &yp);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return loc;
}

fz_colorspace *mupdf_document_output_intent(fz_context *ctx, fz_document *doc, mupdf_error_t **errptr)
{
    fz_colorspace *cs = NULL;
    fz_try(ctx)
    {
        cs = fz_document_output_intent(ctx, doc);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return cs;
}

fz_outline *mupdf_load_outline(fz_context *ctx, fz_document *doc, mupdf_error_t **errptr)
{
    fz_outline *outline = NULL;
    fz_try(ctx)
    {
        outline = fz_load_outline(ctx, doc);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return outline;
}

/* PdfDocument */
pdf_document *mupdf_pdf_open_document_from_bytes(fz_context *ctx, fz_buffer *bytes, mupdf_error_t **errptr)
{
    pdf_document *pdf = NULL;
    fz_stream *stream = NULL;
    fz_var(stream);
    fz_try(ctx)
    {
        stream = fz_open_buffer(ctx, bytes);
        pdf = pdf_open_document_with_stream(ctx, stream);
    }
    fz_always(ctx)
    {
        fz_drop_stream(ctx, stream);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return pdf;
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
    fz_var(buf);
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

pdf_obj *mupdf_pdf_catalog(fz_context *ctx, pdf_document *pdf, mupdf_error_t **errptr)
{
    pdf_obj *obj = NULL;
    fz_try(ctx)
    {
        obj = pdf_dict_get(ctx, pdf_trailer(ctx, pdf), PDF_NAME(Root));
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

pdf_obj *mupdf_pdf_graft_mapped_object(fz_context *ctx, pdf_graft_map *map, pdf_obj *obj, mupdf_error_t **errptr)
{
    pdf_obj *graft_obj = NULL;
    fz_try(ctx)
    {
        graft_obj = pdf_graft_mapped_object(ctx, map, obj);
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
    fz_var(resources);
    fz_var(page_obj);
    fz_var(contents);
    fz_try(ctx)
    {
        // create /Resources and /Contents objects
        resources = pdf_add_object_drop(ctx, pdf, pdf_new_dict(ctx, pdf, 1));
        page_obj = pdf_add_page(ctx, pdf, mediabox, 0, resources, contents);
        pdf_insert_page(ctx, pdf, page_no, page_obj);
        int n = page_no;
        int page_count = pdf_count_pages(ctx, pdf);
        while (n < 0)
        {
            n += page_count;
        }
        fz_page *fz_page = fz_load_page(ctx, &pdf->super, n);
        page = pdf_page_from_fz_page(ctx, fz_page);
    }
    fz_always(ctx)
    {
        fz_drop_buffer(ctx, contents);
        pdf_drop_obj(ctx, page_obj);
        pdf_drop_obj(ctx, resources);
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
        obj = pdf_lookup_page_obj(ctx, pdf, page_no);
        pdf_keep_obj(ctx, obj);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return obj;
}

void mupdf_pdf_insert_page(fz_context *ctx, pdf_document *pdf, int page_no, pdf_obj *page, mupdf_error_t **errptr)
{
    if (page_no < 0 || page_no > pdf_count_pages(ctx, pdf))
    {
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
    if (page_no < 0 || page_no >= pdf_count_pages(ctx, pdf))
    {
        *errptr = mupdf_new_error_from_str("page_no is not a valid page");
        return;
    }
    fz_try(ctx)
    {
        pdf_delete_page(ctx, pdf, page_no);
        if (pdf->rev_page_map)
        {
            pdf_drop_page_tree(ctx, pdf);
        }
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
}

/* Device */
fz_device *mupdf_new_draw_device(fz_context *ctx, fz_pixmap *pixmap, fz_irect clip, mupdf_error_t **errptr)
{
    fz_device *device = NULL;
    fz_try(ctx)
    {
        if (fz_is_infinite_irect(clip))
        {
            device = fz_new_draw_device(ctx, fz_identity, pixmap);
        }
        else
        {
            device = fz_new_draw_device_with_bbox(ctx, fz_identity, pixmap, &clip);
        }
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return device;
}

fz_device *mupdf_new_display_list_device(fz_context *ctx, fz_display_list *list, mupdf_error_t **errptr)
{
    fz_device *device = NULL;
    fz_try(ctx)
    {
        device = fz_new_list_device(ctx, list);
        fz_keep_display_list(ctx, list);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return device;
}

fz_device *mupdf_new_stext_device(fz_context *ctx, fz_stext_page *tp, int flags, mupdf_error_t **errptr)
{
    fz_device *device = NULL;
    fz_try(ctx)
    {
        fz_stext_options opts = {0};
        opts.flags = flags;
        device = fz_new_stext_device(ctx, tp, &opts);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return device;
}

void mupdf_fill_path(fz_context *ctx, fz_device *device, fz_path *path, bool even_odd, fz_matrix ctm, fz_colorspace *cs, const float *color, float alpha, fz_color_params cp, mupdf_error_t **errptr)
{
    fz_try(ctx)
    {
        fz_fill_path(ctx, device, path, even_odd, ctm, cs, color, alpha, cp);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
}

void mupdf_stroke_path(fz_context *ctx, fz_device *device, fz_path *path, fz_stroke_state *stroke, fz_matrix ctm, fz_colorspace *cs, const float *color, float alpha, fz_color_params cp, mupdf_error_t **errptr)
{
    fz_try(ctx)
    {
        fz_stroke_path(ctx, device, path, stroke, ctm, cs, color, alpha, cp);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
}

void mupdf_clip_path(fz_context *ctx, fz_device *device, fz_path *path, bool even_odd, fz_matrix ctm, mupdf_error_t **errptr)
{
    fz_try(ctx)
    {
        fz_clip_path(ctx, device, path, even_odd, ctm, fz_infinite_rect);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
}

void mupdf_clip_stroke_path(fz_context *ctx, fz_device *device, fz_path *path, fz_stroke_state *stroke, fz_matrix ctm, mupdf_error_t **errptr)
{
    fz_try(ctx)
    {
        fz_clip_stroke_path(ctx, device, path, stroke, ctm, fz_infinite_rect);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
}

void mupdf_fill_text(fz_context *ctx, fz_device *device, fz_text *text, fz_matrix ctm, fz_colorspace *cs, const float *color, float alpha, fz_color_params cp, mupdf_error_t **errptr)
{
    fz_try(ctx)
    {
        fz_fill_text(ctx, device, text, ctm, cs, color, alpha, cp);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
}

void mupdf_stroke_text(fz_context *ctx, fz_device *device, fz_text *text, fz_stroke_state *stroke, fz_matrix ctm, fz_colorspace *cs, const float *color, float alpha, fz_color_params cp, mupdf_error_t **errptr)
{
    fz_try(ctx)
    {
        fz_stroke_text(ctx, device, text, stroke, ctm, cs, color, alpha, cp);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
}

void mupdf_clip_text(fz_context *ctx, fz_device *device, fz_text *text, fz_matrix ctm, mupdf_error_t **errptr)
{
    fz_try(ctx)
    {
        fz_clip_text(ctx, device, text, ctm, fz_infinite_rect);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
}

void mupdf_clip_stroke_text(fz_context *ctx, fz_device *device, fz_text *text, fz_stroke_state *stroke, fz_matrix ctm, mupdf_error_t **errptr)
{
    fz_try(ctx)
    {
        fz_clip_stroke_text(ctx, device, text, stroke, ctm, fz_infinite_rect);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
}

void mupdf_ignore_text(fz_context *ctx, fz_device *device, fz_text *text, fz_matrix ctm, mupdf_error_t **errptr)
{
    fz_try(ctx)
    {
        fz_ignore_text(ctx, device, text, ctm);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
}

void mupdf_fill_shade(fz_context *ctx, fz_device *device, fz_shade *shade, fz_matrix ctm, float alpha, fz_color_params cp, mupdf_error_t **errptr)
{
    fz_try(ctx)
    {
        fz_fill_shade(ctx, device, shade, ctm, alpha, cp);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
}

void mupdf_fill_image(fz_context *ctx, fz_device *device, fz_image *image, fz_matrix ctm, float alpha, fz_color_params cp, mupdf_error_t **errptr)
{
    fz_try(ctx)
    {
        fz_fill_image(ctx, device, image, ctm, alpha, cp);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
}

void mupdf_fill_image_mask(fz_context *ctx, fz_device *device, fz_image *image, fz_matrix ctm, fz_colorspace *cs, const float *color, float alpha, fz_color_params cp, mupdf_error_t **errptr)
{
    fz_try(ctx)
    {
        fz_fill_image_mask(ctx, device, image, ctm, cs, color, alpha, cp);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
}

void mupdf_clip_image_mask(fz_context *ctx, fz_device *device, fz_image *image, fz_matrix ctm, mupdf_error_t **errptr)
{
    fz_try(ctx)
    {
        fz_clip_image_mask(ctx, device, image, ctm, fz_infinite_rect);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
}

void mupdf_pop_clip(fz_context *ctx, fz_device *device, mupdf_error_t **errptr)
{
    fz_try(ctx)
    {
        fz_pop_clip(ctx, device);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
}

void mupdf_begin_layer(fz_context *ctx, fz_device *device, const char *name, mupdf_error_t **errptr)
{
    fz_try(ctx)
    {
        fz_begin_layer(ctx, device, name);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
}

void mupdf_end_layer(fz_context *ctx, fz_device *device, mupdf_error_t **errptr)
{
    fz_try(ctx)
    {
        fz_end_layer(ctx, device);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
}

void mupdf_begin_mask(fz_context *ctx, fz_device *device, fz_rect area, bool luminosity, fz_colorspace *cs, const float *color, fz_color_params cp, mupdf_error_t **errptr)
{
    fz_try(ctx)
    {
        fz_begin_mask(ctx, device, area, luminosity, cs, color, cp);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
}

void mupdf_end_mask(fz_context *ctx, fz_device *device, mupdf_error_t **errptr)
{
    fz_try(ctx)
    {
        fz_end_mask(ctx, device);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
}

void mupdf_begin_group(fz_context *ctx, fz_device *device, fz_rect area, fz_colorspace *cs, bool isolated, bool knockout, int blendmode, float alpha, mupdf_error_t **errptr)
{
    fz_try(ctx)
    {
        fz_begin_group(ctx, device, area, cs, isolated, knockout, blendmode, alpha);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
}

void mupdf_end_group(fz_context *ctx, fz_device *device, mupdf_error_t **errptr)
{
    fz_try(ctx)
    {
        fz_end_group(ctx, device);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
}

int mupdf_begin_tile(fz_context *ctx, fz_device *device, fz_rect area, fz_rect view, float xstep, float ystep, fz_matrix ctm, int id, mupdf_error_t **errptr)
{
    int i = 0;
    fz_try(ctx)
    {
        i = fz_begin_tile_id(ctx, device, area, view, xstep, ystep, ctm, id);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return i;
}

void mupdf_end_tile(fz_context *ctx, fz_device *device, mupdf_error_t **errptr)
{
    fz_try(ctx)
    {
        fz_end_tile(ctx, device);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
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

void mupdf_pdf_filter_page_contents(fz_context *ctx, pdf_page *page, pdf_filter_options *filter, mupdf_error_t **errptr)
{
    fz_try(ctx)
    {
        pdf_filter_page_contents(ctx, page->doc, page, filter);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
}

void mupdf_pdf_page_set_rotation(fz_context *ctx, pdf_page *page, int rotation, mupdf_error_t **errptr)
{
    if (rotation % 90)
    {
        *errptr = mupdf_new_error_from_str("rotation not multiple of 90");
        return;
    }
    fz_try(ctx)
    {
        pdf_dict_put_int(ctx, page->obj, PDF_NAME(Rotate), (int64_t)rotation);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
}

void mupdf_pdf_page_set_crop_box(fz_context *ctx, pdf_page *page, fz_rect rect, mupdf_error_t **errptr)
{
    fz_try(ctx)
    {
        fz_rect mediabox = pdf_bound_page(ctx, page);
        pdf_obj *obj = pdf_dict_get_inheritable(ctx, page->obj, PDF_NAME(MediaBox));
        if (obj)
        {
            mediabox = pdf_to_rect(ctx, obj);
        }
        fz_rect cropbox = fz_empty_rect;
        cropbox.x0 = rect.x0;
        cropbox.y0 = mediabox.y1 - rect.y1;
        cropbox.x1 = rect.x1;
        cropbox.y1 = mediabox.y1 - rect.y0;
        pdf_dict_put_drop(ctx, page->obj, PDF_NAME(CropBox), pdf_new_rect(ctx, page->doc, cropbox));
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
}

fz_point mupdf_pdf_page_crop_box_position(fz_context *ctx, pdf_page *page)
{
    fz_point pos = fz_make_point(0, 0);
    pdf_obj *obj = pdf_dict_get_inheritable(ctx, page->obj, PDF_NAME(CropBox));
    if (!obj)
    {
        return pos;
    }
    fz_rect cbox = pdf_to_rect(ctx, obj);
    pos = fz_make_point(cbox.x0, cbox.y0);
    return pos;
}

fz_rect mupdf_pdf_page_media_box(fz_context *ctx, pdf_page *page)
{
    fz_rect r = fz_empty_rect;
    pdf_obj *obj = pdf_dict_get_inheritable(ctx, page->obj, PDF_NAME(MediaBox));
    if (!obj)
    {
        return r;
    }
    r = pdf_to_rect(ctx, obj);
    return r;
}

fz_matrix mupdf_pdf_page_transform(fz_context *ctx, pdf_page *page, mupdf_error_t **errptr)
{
    fz_matrix ctm = fz_identity;
    fz_try(ctx)
    {
        pdf_page_transform(ctx, page, NULL, &ctm);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return ctm;
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

const char *mupdf_pdf_annot_author(fz_context *ctx, pdf_annot *annot, mupdf_error_t **errptr)
{
    const char *author = NULL;
    fz_try(ctx)
    {
        author = pdf_annot_author(ctx, annot);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return author;
}

void mupdf_pdf_set_annot_author(fz_context *ctx, pdf_annot *annot, const char *author, mupdf_error_t **errptr)
{
    fz_try(ctx)
    {
        pdf_set_annot_author(ctx, annot, author);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
}

void mupdf_pdf_filter_annot_contents(fz_context *ctx, pdf_annot *annot, pdf_filter_options *filter, mupdf_error_t **errptr)
{
    fz_try(ctx)
    {
        pdf_filter_annot_contents(ctx, pdf_annot_page(ctx, annot)->doc, annot, filter);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
}

/* DocumentWriter */
fz_document_writer *mupdf_new_document_writer(fz_context *ctx, const char *filename, const char *format, const char *options, mupdf_error_t **errptr)
{
    fz_document_writer *writer = NULL;
    fz_try(ctx)
    {
        writer = fz_new_document_writer(ctx, filename, format, options);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return writer;
}

fz_device *mupdf_document_writer_begin_page(fz_context *ctx, fz_document_writer *writer, fz_rect mediabox, mupdf_error_t **errptr)
{
    fz_device *device = NULL;
    fz_try(ctx)
    {
        device = fz_begin_page(ctx, writer, mediabox);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return device;
}

void mupdf_document_writer_end_page(fz_context *ctx, fz_document_writer *writer, mupdf_error_t **errptr)
{
    fz_try(ctx)
    {
        fz_end_page(ctx, writer);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
}

/* Bitmap */
fz_bitmap *mupdf_new_bitmap_from_pixmap(fz_context *ctx, fz_pixmap *pixmap, mupdf_error_t **errptr)
{
    fz_bitmap *bitmap = NULL;
    fz_try(ctx)
    {
        bitmap = fz_new_bitmap_from_pixmap(ctx, pixmap, NULL);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return bitmap;
}
