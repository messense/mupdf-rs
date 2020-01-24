#include <stdbool.h>

#include "wrapper.h"

fz_pixmap* mupdf_new_pixmap(fz_context* ctx, fz_colorspace* cs, int x, int y, int w, int h, bool alpha) {
    fz_pixmap *pixmap = NULL;
    fz_try(ctx) {
        pixmap = fz_new_pixmap(ctx, cs, w, h, NULL, alpha);
		pixmap->x = x;
		pixmap->y = y;
    }
    fz_catch(ctx) {
        // FIXME
    }
    return pixmap;
}

void mupdf_clear_pixmap(fz_context* ctx, fz_pixmap* pixmap) {
    fz_try(ctx) {
		fz_clear_pixmap(ctx, pixmap);
    }
	fz_catch(ctx) {
		// FIXME
    }
}