#include <stdbool.h>
#include <string.h>
#include <assert.h>
#ifdef _WIN32
#include <windows.h>
#else
#include <pthread.h>
#endif

#include "wrapper.h"

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
            font = fz_new_font_from_memory(ctx, name, data, size, index, 0);
        else
            font = fz_new_font_from_file(ctx, name, name, index, 0);
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