#ifndef TGSKK_H
#define TGSKK_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct TgskkZenz TgskkZenz;
typedef struct TgskkSkkserv TgskkSkkserv;
typedef struct TgskkRuntime TgskkRuntime;
typedef struct TgskkSession TgskkSession;

/* All handles and their functions must stay on the creating thread. Sessions
 * share dictionary data, but never share composition state. A runtime may be
 * freed after its sessions have been created. Non-null pointers must be valid;
 * strings passed in must be NUL-terminated UTF-8. Free each handle exactly
 * once. */
TgskkRuntime *tgskk_runtime_new(const char *dictionary, const char *encoding,
                                const char *user_dictionary, char **error);
void tgskk_runtime_free(TgskkRuntime *runtime);
/* Save is for the host's save/shutdown hook, never the key-event path. */
uint8_t tgskk_runtime_save(TgskkRuntime *runtime, char **error);
void tgskk_string_free(char *string);

/* Error outputs are newly allocated strings, or NULL on success, and must be
 * released with tgskk_string_free. NULL user_dictionary disables persistence.
 */
TgskkSession *tgskk_session_new(const TgskkRuntime *runtime, char **error);
void tgskk_session_free(TgskkSession *session);

/* Loading and scoring run off the input thread. Status: 0 loading, 1 ready,
 * 2 unavailable. A session retains the worker after this handle is freed.
 * NULL in set_zenz disables assistance without changing composition. */
TgskkZenz *tgskk_zenz_new(const char *executable, const char *model, char **error);
void tgskk_zenz_free(TgskkZenz *zenz);
uint8_t tgskk_zenz_status(const TgskkZenz *zenz);
/* Each error is returned once. Free non-NULL strings with tgskk_string_free. */
char *tgskk_zenz_take_error(const TgskkZenz *zenz);
uint8_t tgskk_session_set_zenz(TgskkSession *session, const TgskkZenz *zenz);
/* Call before each key with bounded text strictly left of the caret. enabled=0
 * invalidates pending work and clears context, including for sensitive fields.
 * Neither function performs inference or waits for the worker. */
uint8_t tgskk_session_context(TgskkSession *session, const char *left, uint8_t enabled);

/* SKK server workers start idle. Only conversions with no local candidates
 * may wait (0..1000 ms); DNS, network and helper IPC run on the worker thread.
 * Google uses a lazily spawned --stdio helper with UTF-8; custom servers use
 * euc-jp or utf-8. Only the reading is transmitted, never left context.
 * Sessions retain the worker after the handle is freed. NULL in set_skkserv
 * disables it without changing composition or the local AI provider. */
TgskkSkkserv *tgskk_skkserv_google_new(const char *executable, uint32_t wait_ms,
                                     char **error);
TgskkSkkserv *tgskk_skkserv_custom_new(const char *host, uint16_t port,
                                     const char *encoding, uint32_t wait_ms,
                                     char **error);
void tgskk_skkserv_free(TgskkSkkserv *server);
char *tgskk_skkserv_take_error(const TgskkSkkserv *server);
uint8_t tgskk_session_set_skkserv(TgskkSession *session, const TgskkSkkserv *server);
/* Permission starts disabled and is revoked by reset, unhandled keys, and
 * set_skkserv. Set before each key according to focus/sensitivity/selection,
 * independently of AI context. Disabled permission also bypasses the cache. */
uint8_t tgskk_session_network_allowed(TgskkSession *session, uint8_t allowed);

typedef struct TgskkView {
  uint8_t consumed;
  uint8_t mode_changed;
  uint8_t selecting;
  uint8_t zenz_applied;
  size_t selected;
  size_t candidate_count;
  const char *committed;
  const char *preedit;
  const char *mode_label;
} TgskkView;

/* Returned views and strings are borrowed until the next key/select/reset/free
 * on that session. NULL means failure; read session_error and discard the
 * session. A Rust panic poisons the session and is contained at this boundary.
 * Modifiers use fcitx5's SimpleMask (XKB bits); keysym is rawKey().sym().
 * Release events are unconsumed and leave the composition unchanged. */
const TgskkView *tgskk_session_key(TgskkSession *session, uint32_t keysym,
                                   uint32_t modifiers, uint8_t release);
const TgskkView *tgskk_session_select(TgskkSession *session, size_t index);
/* Move the highlighted candidate without committing it. */
const TgskkView *tgskk_session_choose(TgskkSession *session, size_t index);
/* Discards all uncommitted input, including nested registration, on focus loss.
 */
const TgskkView *tgskk_session_reset(TgskkSession *session);
const TgskkView *tgskk_session_view(const TgskkSession *session);
const char *tgskk_session_error(const TgskkSession *session);
const char *tgskk_candidate_text(const TgskkSession *session, size_t index);
const char *tgskk_candidate_annotation(const TgskkSession *session,
                                       size_t index);

#ifdef __cplusplus
}
#endif
#endif
