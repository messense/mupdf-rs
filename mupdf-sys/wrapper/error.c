#include "internal.h"

static mupdf_error_t mupdf_oom_error = { FZ_ERROR_SYSTEM, "out of memory" };

void mupdf_save_error(fz_context *ctx, mupdf_error_t **errptr)
{
    assert(errptr != NULL);
    if (*errptr)
        mupdf_drop_error(*errptr);
    int type = fz_caught(ctx);
    const char *message = fz_caught_message(ctx);
    mupdf_error_t *err = malloc(sizeof(mupdf_error_t));
    if (!err)
    {
        *errptr = &mupdf_oom_error;
        return;
    }
    err->type = type;
    err->message = strdup(message ? message : "");
    if (!err->message)
    {
        free(err);
        *errptr = &mupdf_oom_error;
        return;
    }
    *errptr = err;
}

mupdf_error_t *mupdf_new_error_from_str(const char *message)
{
    mupdf_error_t *err = malloc(sizeof(mupdf_error_t));
    if (!err)
        return &mupdf_oom_error;
    err->type = FZ_ERROR_GENERIC;
    err->message = strdup(message ? message : "");
    if (!err->message)
    {
        free(err);
        return &mupdf_oom_error;
    }
    return err;
}

void mupdf_drop_error(mupdf_error_t *err)
{
    if (err == NULL || err == &mupdf_oom_error)
    {
        return;
    }
    if (err->message != NULL)
    {
        free(err->message);
    }
    free(err);
}

void mupdf_drop_str(char *s)
{
    if (s != NULL)
    {
        free(s);
    }
}