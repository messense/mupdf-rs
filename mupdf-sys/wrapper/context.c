#include "internal.h"

#if defined(__APPLE__) && defined(__ENVIRONMENT_MAC_OS_X_VERSION_MIN_REQUIRED__) && \
    __ENVIRONMENT_MAC_OS_X_VERSION_MIN_REQUIRED__ >= 101200
#include <os/lock.h>
#define MUPDF_USE_UNFAIR_LOCK
#endif

/* Locking for MuPDF contexts.
 *
 * MuPDF hands the `user` pointer of the fz_locks_context back to the lock
 * callbacks, and clones inherit their base context's locks. Each base context
 * therefore owns a lock set: contexts of different families contend only on
 * FZ_LOCK_FREETYPE, and tearing down one family cannot touch the mutexes of
 * another. */

#ifdef _WIN32
typedef CRITICAL_SECTION wrapper_mutex;
#elif defined(MUPDF_USE_UNFAIR_LOCK)
/* Avoid false sharing between hot locks. */
typedef union
{
    os_unfair_lock lock;
    char padding[128];
} wrapper_mutex;
#else
typedef pthread_mutex_t wrapper_mutex;
#endif

typedef struct
{
    wrapper_mutex mutexes[FZ_LOCK_MAX];
} wrapper_lock_set;

/* MuPDF passes the context to OpenJPEG and HarfBuzz callbacks through
 * process-wide globals guarded by FZ_LOCK_FREETYPE (load-jpx.c, harfbuzz.c),
 * so that lock must be one mutex across all families. */
#ifdef _WIN32
static SRWLOCK freetype_lock = SRWLOCK_INIT;
#else
static pthread_mutex_t freetype_lock = PTHREAD_MUTEX_INITIALIZER;
#endif

static void lock(void *user, int lock)
{
    wrapper_lock_set *ls = (wrapper_lock_set *)user;
    if (lock == FZ_LOCK_FREETYPE)
    {
#ifdef _WIN32
        AcquireSRWLockExclusive(&freetype_lock);
#else
        (void)pthread_mutex_lock(&freetype_lock);
#endif
        return;
    }
#ifdef _WIN32
    EnterCriticalSection(&ls->mutexes[lock]);
#elif defined(MUPDF_USE_UNFAIR_LOCK)
    os_unfair_lock_lock(&ls->mutexes[lock].lock);
#else
    (void)pthread_mutex_lock(&ls->mutexes[lock]);
#endif
}

static void unlock(void *user, int lock)
{
    wrapper_lock_set *ls = (wrapper_lock_set *)user;
    if (lock == FZ_LOCK_FREETYPE)
    {
#ifdef _WIN32
        ReleaseSRWLockExclusive(&freetype_lock);
#else
        (void)pthread_mutex_unlock(&freetype_lock);
#endif
        return;
    }
#ifdef _WIN32
    LeaveCriticalSection(&ls->mutexes[lock]);
#elif defined(MUPDF_USE_UNFAIR_LOCK)
    os_unfair_lock_unlock(&ls->mutexes[lock].lock);
#else
    (void)pthread_mutex_unlock(&ls->mutexes[lock]);
#endif
}

static wrapper_lock_set *new_lock_set(void)
{
    int i;
    wrapper_lock_set *ls = (wrapper_lock_set *)malloc(sizeof(*ls));
    if (!ls)
        return NULL;
    for (i = 0; i < FZ_LOCK_MAX; i++)
    {
#ifdef _WIN32
        InitializeCriticalSection(&ls->mutexes[i]);
#elif defined(MUPDF_USE_UNFAIR_LOCK)
        ls->mutexes[i].lock = (os_unfair_lock)OS_UNFAIR_LOCK_INIT;
#else
        (void)pthread_mutex_init(&ls->mutexes[i], NULL);
#endif
    }
    return ls;
}

static void drop_lock_set(wrapper_lock_set *ls)
{
    int i;
    if (!ls)
        return;
    for (i = 0; i < FZ_LOCK_MAX; i++)
    {
#ifdef _WIN32
        DeleteCriticalSection(&ls->mutexes[i]);
#elif !defined(MUPDF_USE_UNFAIR_LOCK)
        (void)pthread_mutex_destroy(&ls->mutexes[i]);
#endif
    }
    free(ls);
}

/* The lock set shared by every context in a family (a base context and its
 * clones). Identifies the family: two contexts may share MuPDF resources
 * only if this is equal for both. */
void *mupdf_context_lock_set(fz_context *ctx)
{
    return ctx ? ctx->locks.user : NULL;
}

/* Context */

/* Drop a context created by mupdf_new_base_context.
 *
 * The lock set is freed only when `ctx` is the base context and no clone of
 * it is alive, because that is the only moment no other thread can be inside
 * the locks. A base context dropped while clones still exist leaks its lock
 * set (three mutexes) rather than risk destroying them under a live clone;
 * drop the clones first with fz_drop_context to avoid it. */
void mupdf_drop_base_context(fz_context *ctx)
{
    wrapper_lock_set *ls;
    int last;

    if (!ctx)
        return;

    ls = (wrapper_lock_set *)ctx->locks.user;
    last = (ctx->master == ctx && ctx->context_count == 1);
    fz_drop_context(ctx);
    if (last)
        drop_lock_set(ls);
}

fz_context *mupdf_new_base_context(size_t max_store)
{
    fz_context *ctx;
    fz_locks_context locks;
    wrapper_lock_set *ls = new_lock_set();
    if (!ls)
        return NULL;
    locks.user = ls;
    locks.lock = lock;
    locks.unlock = unlock;

    if (max_store == 0)
        max_store = FZ_STORE_DEFAULT;
    ctx = fz_new_context(NULL, &locks, max_store);
    if (!ctx)
    {
        drop_lock_set(ls);
        return NULL;
    }
    fz_try(ctx) {
        fz_register_document_handlers(ctx);
    }
    fz_catch(ctx) {
        mupdf_drop_base_context(ctx);
        return NULL;
    }
    // Disable default warning & error printing
    fz_set_warning_callback(ctx, NULL, NULL);
    fz_set_error_callback(ctx, NULL, NULL);
    return ctx;
}
