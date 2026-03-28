#include "internal.h"

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
    TRY_CATCH(pdf_obj*, NULL, pdf_add_object(ctx, pdf, obj));
}

pdf_obj *mupdf_pdf_add_stream(fz_context *ctx, pdf_document *pdf, fz_buffer *buf, pdf_obj *obj, int compressed, mupdf_error_t **errptr)
{
    TRY_CATCH(pdf_obj*, NULL, pdf_add_stream(ctx, pdf, buf, obj, compressed));
}

pdf_obj *mupdf_pdf_create_object(fz_context *ctx, pdf_document *pdf, mupdf_error_t **errptr)
{
    TRY_CATCH(pdf_obj*, NULL, pdf_new_indirect(ctx, pdf, pdf_create_object(ctx, pdf), 0));
}

void mupdf_pdf_delete_object(fz_context *ctx, pdf_document *pdf, int num, mupdf_error_t **errptr)
{
    TRY_CATCH_VOID(pdf_delete_object(ctx, pdf, num));
}

pdf_obj *mupdf_pdf_add_image(fz_context *ctx, pdf_document *pdf, fz_image *image, mupdf_error_t **errptr)
{
    TRY_CATCH(pdf_obj*, NULL, pdf_add_image(ctx, pdf, image));
}

pdf_obj *mupdf_pdf_add_font(fz_context *ctx, pdf_document *pdf, fz_font *font, mupdf_error_t **errptr)
{
    TRY_CATCH(pdf_obj*, NULL, pdf_add_cid_font(ctx, pdf, font));
}

pdf_obj *mupdf_pdf_add_cjk_font(fz_context *ctx, pdf_document *pdf, fz_font *font, int ordering, int wmode, bool serif, mupdf_error_t **errptr)
{
    TRY_CATCH(pdf_obj*, NULL, pdf_add_cjk_font(ctx, pdf, font, ordering, wmode, serif));
}

pdf_obj *mupdf_pdf_add_simple_font(fz_context *ctx, pdf_document *pdf, fz_font *font, int encoding, mupdf_error_t **errptr)
{
    TRY_CATCH(pdf_obj*, NULL, pdf_add_simple_font(ctx, pdf, font, encoding));
}

void mupdf_pdf_save_document(fz_context *ctx, pdf_document *pdf, const char *filename, pdf_write_options pwo, mupdf_error_t **errptr)
{
    TRY_CATCH_VOID(pdf_save_document(ctx, pdf, filename, &pwo));
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
    TRY_CATCH_VOID(pdf_enable_js(ctx, pdf));
}

void mupdf_pdf_disable_js(fz_context *ctx, pdf_document *pdf, mupdf_error_t **errptr)
{
    TRY_CATCH_VOID(pdf_disable_js(ctx, pdf));
}

bool mupdf_pdf_js_supported(fz_context *ctx, pdf_document *pdf, mupdf_error_t **errptr)
{
    TRY_CATCH(bool, false, pdf_js_supported(ctx, pdf));
}

void mupdf_pdf_calculate_form(fz_context *ctx, pdf_document *pdf, mupdf_error_t **errptr)
{
    if (!pdf->recalculate) return;

    TRY_CATCH_VOID(pdf_calculate_form(ctx, pdf));
}

void mupdf_pdf_bake_document(fz_context *ctx, pdf_document *pdf, bool bake_annots, bool bake_widgets, mupdf_error_t **errptr)
{
    TRY_CATCH_VOID(pdf_bake_document(ctx, pdf, bake_annots, bake_widgets));
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

pdf_obj *mupdf_pdf_load_name_tree(fz_context *ctx, pdf_document *pdf, pdf_obj* name, mupdf_error_t **errptr)
{
    pdf_obj *obj = NULL;
    fz_try(ctx)
    {
        obj = pdf_load_name_tree(ctx, pdf, name);
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
    TRY_CATCH(int, 0, pdf_xref_len(ctx, pdf));
}

pdf_graft_map *mupdf_pdf_new_graft_map(fz_context *ctx, pdf_document *pdf, mupdf_error_t **errptr)
{
    TRY_CATCH(pdf_graft_map*, NULL, pdf_new_graft_map(ctx, pdf));
}

pdf_obj *mupdf_pdf_graft_object(fz_context *ctx, pdf_document *doc, pdf_obj *obj, mupdf_error_t **errptr)
{
    TRY_CATCH(pdf_obj*, NULL, pdf_graft_object(ctx, doc, obj));
}

pdf_obj *mupdf_pdf_graft_mapped_object(fz_context *ctx, pdf_graft_map *map, pdf_obj *obj, mupdf_error_t **errptr)
{
    TRY_CATCH(pdf_obj*, NULL, pdf_graft_mapped_object(ctx, map, obj));
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
    TRY_CATCH_VOID(pdf_insert_page(ctx, pdf, page_no, page));
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

void mupdf_pdf_graft_page(fz_context *ctx, pdf_document *dst, int page_to, pdf_document *src, int page_from, mupdf_error_t **errptr)
{
    if (page_from < 0 || page_from >= pdf_count_pages(ctx, src))
    {
        *errptr = mupdf_new_error_from_str("source page index is not valid");
        return;
    }
    if (page_to < 0 || page_to > pdf_count_pages(ctx, dst))
    {
        *errptr = mupdf_new_error_from_str("target page index is not valid");
        return;
    }
    TRY_CATCH_VOID(pdf_graft_page(ctx, dst, page_to, src, page_from));
}

void mupdf_pdf_delete_page_range(fz_context *ctx, pdf_document *pdf, int start, int end, mupdf_error_t **errptr)
{
    int page_count = pdf_count_pages(ctx, pdf);
    if (start < 0 || end < start || end > page_count)
    {
        *errptr = mupdf_new_error_from_str("page range is not valid");
        return;
    }
    TRY_CATCH_VOID(pdf_delete_page_range(ctx, pdf, start, end));
}

void mupdf_pdf_page_label(fz_context *ctx, pdf_document *pdf, int page_no, char *buf, size_t size, mupdf_error_t **errptr)
{
    if (page_no < 0 || page_no >= pdf_count_pages(ctx, pdf))
    {
        *errptr = mupdf_new_error_from_str("page_no is not a valid page");
        return;
    }
    TRY_CATCH_VOID(pdf_page_label(ctx, pdf, page_no, buf, size));
}

void mupdf_pdf_set_page_labels(fz_context *ctx, pdf_document *pdf, int index, pdf_page_label_style style, const char *prefix, int start, mupdf_error_t **errptr)
{
    if (index < 0 || index > pdf_count_pages(ctx, pdf))
    {
        *errptr = mupdf_new_error_from_str("page label index is not valid");
        return;
    }
    TRY_CATCH_VOID(pdf_set_page_labels(ctx, pdf, index, style, prefix, start));
}

void mupdf_pdf_begin_operation(fz_context *ctx, pdf_document *doc, const char *operation, mupdf_error_t **errptr)
{
    TRY_CATCH_VOID(pdf_begin_operation(ctx, doc, operation));
}

void mupdf_pdf_end_operation(fz_context *ctx, pdf_document *doc, mupdf_error_t **errptr)
{
    TRY_CATCH_VOID(pdf_end_operation(ctx, doc));
}

void mupdf_pdf_abandon_operation(fz_context *ctx, pdf_document *doc, mupdf_error_t **errptr)
{
    TRY_CATCH_VOID(pdf_abandon_operation(ctx, doc));
}

int mupdf_pdf_count_layer_configs(fz_context *ctx, pdf_document *pdf, mupdf_error_t **errptr)
{
	TRY_CATCH(int, 0, pdf_count_layer_configs(ctx, pdf));
}

int mupdf_pdf_count_layers(fz_context *ctx, pdf_document *pdf, mupdf_error_t **errptr)
{
	TRY_CATCH(int, 0, pdf_count_layers(ctx, pdf));
}

const char *mupdf_pdf_layer_name(fz_context *ctx, pdf_document *pdf, int layer, mupdf_error_t **errptr)
{
	if(layer < 0 || layer > mupdf_pdf_count_layers(ctx, pdf, errptr)) {
        *errptr = mupdf_new_error_from_str("layer num is not in range");
        return "";
	}
	TRY_CATCH(const char *, NULL, pdf_layer_name(ctx, pdf, layer));
}


int mupdf_pdf_layer_is_enabled(fz_context *ctx, pdf_document *pdf, int layer, mupdf_error_t **errptr)
{
	if(layer < 0 || layer > mupdf_pdf_count_layers(ctx, pdf, errptr)) {
        *errptr = mupdf_new_error_from_str("layer num is not in range");
        return 0;
	}
	TRY_CATCH(int, 0, pdf_layer_is_enabled(ctx, pdf, layer));
}

void mupdf_pdf_enable_layer(fz_context *ctx, pdf_document *pdf, int layer, int enabled, mupdf_error_t **errptr)
{
	if(layer < 0 || layer > mupdf_pdf_count_layers(ctx, pdf, errptr)) {
        *errptr = mupdf_new_error_from_str("layer num is not in range");
        return;
	}
	TRY_CATCH_VOID(pdf_enable_layer(ctx, pdf, layer, enabled));
}

void mupdf_pdf_layer_config_info(fz_context *ctx, pdf_document *pdf, int config_num, pdf_layer_config *info, mupdf_error_t **errptr)
{
	if(config_num < 0 || config_num > mupdf_pdf_count_layer_configs(ctx, pdf, errptr)) {
        *errptr = mupdf_new_error_from_str("config_num is not in range");
        return;
	}
	TRY_CATCH_VOID(pdf_layer_config_info(ctx, pdf, config_num, info));
}

const char *mupdf_pdf_layer_config_creator(fz_context *ctx, pdf_document *pdf, int config_num, mupdf_error_t **errptr)
{
	if(config_num < 0 || config_num > mupdf_pdf_count_layer_configs(ctx, pdf, errptr)) {
        *errptr = mupdf_new_error_from_str("config_num is not in range");
        return "";
	}
	TRY_CATCH(const char *, NULL, pdf_layer_config_creator(ctx, pdf, config_num));
}

const char *mupdf_pdf_layer_config_name(fz_context *ctx, pdf_document *pdf, int config_num, mupdf_error_t **errptr)
{
	if(config_num < 0 || config_num > mupdf_pdf_count_layer_configs(ctx, pdf, errptr)) {
        *errptr = mupdf_new_error_from_str("config_num is not in range");
        return "";
	}
	TRY_CATCH(const char *, NULL, pdf_layer_config_name(ctx, pdf, config_num));
}

void mupdf_pdf_select_layer_config(fz_context *ctx, pdf_document *pdf, int config_num, mupdf_error_t **errptr)
{
	if(config_num < 0 || config_num > mupdf_pdf_count_layer_configs(ctx, pdf, errptr)) {
        *errptr = mupdf_new_error_from_str("config_num is not in range");
        return;
	}
	TRY_CATCH_VOID(pdf_select_layer_config(ctx, pdf, config_num));
}

int mupdf_pdf_count_layer_config_ui(fz_context *ctx, pdf_document *pdf, mupdf_error_t **errptr)
{
	TRY_CATCH(int, 0, pdf_count_layer_config_ui(ctx, pdf));
}

void mupdf_pdf_select_layer_config_ui(fz_context *ctx, pdf_document *pdf, int ui, mupdf_error_t **errptr)
{
	TRY_CATCH_VOID(pdf_select_layer_config_ui(ctx, pdf, ui));
}

void mupdf_pdf_deselect_layer_config_ui(fz_context *ctx, pdf_document *pdf, int ui, mupdf_error_t **errptr)
{
	TRY_CATCH_VOID(pdf_deselect_layer_config_ui(ctx, pdf, ui));
}

void mupdf_pdf_toggle_layer_config_ui(fz_context *ctx, pdf_document *pdf, int ui, mupdf_error_t **errptr)
{
	TRY_CATCH_VOID(pdf_toggle_layer_config_ui(ctx, pdf, ui));
}

const char *mupdf_pdf_layer_config_ui_type_to_string(fz_context *ctx, pdf_layer_config_ui_type type, mupdf_error_t **errptr)
{
	TRY_CATCH(const char *, NULL, pdf_layer_config_ui_type_to_string(type));
}

pdf_layer_config_ui_type mupdf_pdf_layer_config_ui_type_from_string(fz_context *ctx, const char *str, mupdf_error_t **errptr)
{
	TRY_CATCH(pdf_layer_config_ui_type, 0, pdf_layer_config_ui_type_from_string(str));
}

void mupdf_pdf_layer_config_ui_info(fz_context *ctx, pdf_document *pdf, int ui, pdf_layer_config_ui *info, mupdf_error_t **errptr) 
{
	TRY_CATCH_VOID(pdf_layer_config_ui_info(ctx, pdf, ui, info));
};

void mupdf_pdf_set_layer_config_as_default(fz_context *ctx, pdf_document *pdf, mupdf_error_t **errptr) {
	TRY_CATCH_VOID(pdf_set_layer_config_as_default(ctx, pdf));
}
