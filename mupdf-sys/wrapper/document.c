#include "internal.h"

/* Document */
fz_document *mupdf_open_document(fz_context *ctx, const char *filename, mupdf_error_t **errptr)
{
    TRY_CATCH(fz_document*, NULL, fz_open_document(ctx, filename));
}

fz_document *mupdf_open_document_from_bytes(fz_context *ctx, fz_buffer *bytes, const char *magic, mupdf_error_t **errptr)
{
    if (!magic)
    {
        return NULL;
    }
    fz_document *doc = NULL;
    fz_stream *stream = NULL;
    fz_var(doc);
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
        fz_drop_document(ctx, doc);
        doc = NULL;
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
    TRY_CATCH(bool, false, fz_recognize_document(ctx, magic) != NULL);
}

bool mupdf_needs_password(fz_context *ctx, fz_document *doc, mupdf_error_t **errptr)
{
    TRY_CATCH(bool, false, fz_needs_password(ctx, doc));
}

bool mupdf_authenticate_password(fz_context *ctx, fz_document *doc, const char *password, mupdf_error_t **errptr)
{
    TRY_CATCH(bool, false, fz_authenticate_password(ctx, doc, password));
}

int mupdf_document_page_count(fz_context *ctx, fz_document *doc, mupdf_error_t **errptr)
{
    TRY_CATCH(int, 0, fz_count_pages(ctx, doc));
}

char *mupdf_lookup_metadata(fz_context *ctx, fz_document *doc, const char *key, mupdf_error_t **errptr)
{
    int len;
    char *value = NULL;
    fz_var(value);
    fz_try(ctx)
    {
        len = fz_lookup_metadata(ctx, doc, key, NULL, 0) + 1;
        if (len > 1)
        {
            value = calloc(len, sizeof(char));
            if (!value)
                fz_throw(ctx, FZ_ERROR_SYSTEM, "cannot allocate metadata buffer");
            fz_lookup_metadata(ctx, doc, key, value, len);
        }
    }
    fz_catch(ctx)
    {
        if (value)
            free(value);
        value = NULL;
        mupdf_save_error(ctx, errptr);
    }
    return value;
}

bool mupdf_is_document_reflowable(fz_context *ctx, fz_document *doc, mupdf_error_t **errptr)
{
    TRY_CATCH(bool, false, fz_is_document_reflowable(ctx, doc));
}

void mupdf_layout_document(fz_context *ctx, fz_document *doc, float w, float h, float em, mupdf_error_t **errptr)
{
    /* Never return from inside fz_try: it would leak the exception-stack slot
       pushed by fz_push_try and leave a stale jump buffer on the context. */
    bool reflowable = false;
    fz_var(reflowable);
    fz_try(ctx)
    {
        reflowable = fz_is_document_reflowable(ctx, doc);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
        return;
    }
    if (!reflowable)
    {
        return;
    }
    if (w <= 0.0f || h <= 0.0f)
    {
        *errptr = mupdf_new_error_from_str("invalid width or height");
        return;
    }
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
    TRY_CATCH(fz_page*, NULL, fz_load_page(ctx, doc, page_no));
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
    fz_page *page = NULL;
    pdf_obj *page_obj = NULL;
    fz_var(dev);
    fz_var(contents);
    fz_var(resources);
    fz_var(page);
    fz_var(page_obj);
    fz_try(ctx)
    {
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
                page_obj = pdf_add_page(ctx, pdfout, mediabox, rotate, resources, contents);
                pdf_insert_page(ctx, pdfout, -1, page_obj);
                pdf_drop_obj(ctx, page_obj);
                page_obj = NULL;
            }
            fz_always(ctx)
            {
                pdf_drop_obj(ctx, resources);
                resources = NULL;
                fz_drop_buffer(ctx, contents);
                contents = NULL;
                fz_drop_device(ctx, dev);
                dev = NULL;
                pdf_drop_obj(ctx, page_obj);
                page_obj = NULL;
                fz_drop_page(ctx, page);
                page = NULL;
            }
            fz_catch(ctx)
            {
                fz_rethrow(ctx);
            }
        }
    }
    fz_catch(ctx)
    {
        pdf_drop_document(ctx, pdfout);
        fz_rethrow(ctx);
    }
    return pdfout;
}

pdf_document *mupdf_convert_to_pdf(fz_context *ctx, fz_document *doc, int fp, int tp, int rotate, fz_cookie *cookie, mupdf_error_t **errptr)
{
    pdf_document *pdfout = NULL;

    if (rotate % 90)
    {
        *errptr = mupdf_new_error_from_str("rotation not multiple of 90");
        return NULL;
    }

    fz_var(pdfout);
    fz_try(ctx)
    {
        pdfout = mupdf_convert_to_pdf_internal(ctx, doc, fp, tp, rotate, cookie);
    }
    fz_catch(ctx)
    {
        pdf_drop_document(ctx, pdfout);
        pdfout = NULL;
        mupdf_save_error(ctx, errptr);
    }
    return pdfout;
}

fz_location mupdf_resolve_link(fz_context *ctx, fz_document *doc, const char *uri, float *xp, float *yp, mupdf_error_t **errptr)
{
    fz_location ret = { -1, -1 };
    fz_try(ctx)
    {
        ret = fz_resolve_link(ctx, doc, uri, xp, yp);
    }
    fz_catch(ctx)
    {
        mupdf_save_error(ctx, errptr);
    }
    return ret;
}


fz_link_dest mupdf_resolve_link_dest(fz_context *ctx, fz_document *doc, const char *uri, mupdf_error_t **errptr)
{
    TRY_CATCH(fz_link_dest, {}, fz_resolve_link_dest(ctx, doc, uri));
}

fz_colorspace *mupdf_document_output_intent(fz_context *ctx, fz_document *doc, mupdf_error_t **errptr)
{
    TRY_CATCH(fz_colorspace*, NULL, fz_document_output_intent(ctx, doc));
}

fz_outline *mupdf_load_outline(fz_context *ctx, fz_document *doc, mupdf_error_t **errptr)
{
    TRY_CATCH(fz_outline*, NULL, fz_load_outline(ctx, doc));
}
