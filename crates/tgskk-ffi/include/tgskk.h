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

enum TgskkModifier {
  TGSKK_MOD_SHIFT = 1,
  TGSKK_MOD_CONTROL = 4,
  TGSKK_MOD_ALT = 8
};
enum TgskkKey {
  TGSKK_KEY_BACKSPACE = 0xff08,
  TGSKK_KEY_TAB = 0xff09,
  TGSKK_KEY_RETURN = 0xff0d,
  TGSKK_KEY_ESCAPE = 0xff1b,
  TGSKK_KEY_LEFT = 0xff51,
  TGSKK_KEY_UP = 0xff52,
  TGSKK_KEY_RIGHT = 0xff53,
  TGSKK_KEY_DOWN = 0xff54,
  TGSKK_KEY_PAGE_UP = 0xff55,
  TGSKK_KEY_PAGE_DOWN = 0xff56
};

/* All handles and their functions must stay on the creating thread. Sessions
 * share dictionary data, but never share composition state. A runtime may be
 * freed after its sessions have been created. Non-null pointers must be valid;
 * strings passed in must be NUL-terminated UTF-8. Free each handle exactly
 * once. */
TgskkRuntime *tgskk_runtime_new(const char *dictionary, const char *encoding,
                                const char *user_dictionary, char **error);
/* Create a runtime with another base dictionary, sharing the existing user
 * dictionary and its saver. Both runtimes stay on the same owner thread. */
TgskkRuntime *tgskk_runtime_with_dictionary(const TgskkRuntime *runtime,
                                            const char *dictionary,
                                            const char *encoding, char **error);
void tgskk_runtime_free(TgskkRuntime *runtime);
/* Startup only, before creating sessions. Adds a read-only user snapshot in
 * front of the base dictionary when another broker owns the writer lock. */
uint8_t tgskk_runtime_prepend_readonly(TgskkRuntime *runtime, const char *path,
                                       const char *encoding, char **error);
typedef struct TgskkSaver TgskkSaver;
/* Create once at startup: copies the initial user dictionary, then transfers
 * coalesced entry changes to a single disk writer. tick never waits for disk.
 * All saver calls stay on the runtime's owner thread; it may outlive runtime.
 * Never combine with runtime_save or another writer of the same file.
 * idle returns 1 when the latest changes are durable, 0 while work remains.
 * free joins the writer; use only at shutdown, after a successful idle check.
 */
TgskkSaver *tgskk_runtime_saver_new(const TgskkRuntime *runtime, char **error);
uint8_t tgskk_saver_tick(TgskkSaver *saver, char **error);
uint8_t tgskk_saver_idle(const TgskkSaver *saver);
void tgskk_saver_free(TgskkSaver *saver);
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
TgskkZenz *tgskk_zenz_new(const char *executable, const char *model,
                          char **error);
void tgskk_zenz_free(TgskkZenz *zenz);
uint8_t tgskk_zenz_status(const TgskkZenz *zenz);
/* Each error is returned once. Free non-NULL strings with tgskk_string_free. */
char *tgskk_zenz_take_error(const TgskkZenz *zenz);
uint8_t tgskk_session_set_zenz(TgskkSession *session, const TgskkZenz *zenz);
/* Call before each key with bounded text strictly left of the caret. enabled=0
 * invalidates pending work and clears context, including for sensitive fields.
 * Neither function performs inference or waits for the worker. */
uint8_t tgskk_session_context(TgskkSession *session, const char *left,
                              uint8_t enabled);

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
uint8_t tgskk_session_set_skkserv(TgskkSession *session,
                                  const TgskkSkkserv *server);
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

/* Side-effect-free eligibility query: 1 means call key(), whose consumed flag
 * remains authoritative; commands can have effects while passing a key through.
 * 0 means ineligible, -1 means failure: read session_error and discard session.
 * Does not advance input or invalidate a borrowed view; panic poisons session.
 */
int8_t tgskk_session_may_process(TgskkSession *session, uint32_t keysym,
                                 uint32_t modifiers);
/* Returned views and strings are borrowed until the next key/select/reset/free
 * on that session. NULL means failure; read session_error and discard the
 * session. A Rust panic poisons the session and is contained at this boundary.
 * Modifiers use XKB bits; keysym uses X11/XKB numeric values (ASCII for text).
 * Release events are unconsumed and leave the composition unchanged. */
const TgskkView *tgskk_session_key(TgskkSession *session, uint32_t keysym,
                                   uint32_t modifiers, uint8_t release);
const TgskkView *tgskk_session_select(TgskkSession *session, size_t index);
/* Move the highlighted candidate without committing it. */
const TgskkView *tgskk_session_choose(TgskkSession *session, size_t index);
/* Discards all uncommitted input, including nested registration, on focus loss.
 */
const TgskkView *tgskk_session_reset(TgskkSession *session);
/* Discard composition and set mode: 0 hiragana, 1 katakana, 2 half-width kana,
 * 3 full-width ASCII, 4 ASCII. Invalid values fail without changing input. */
const TgskkView *tgskk_session_set_mode(TgskkSession *session, uint8_t mode);
/* Host capability, default off. For zero-wait server requests, preserve reading
 * and okuri while fetching rather than enter registration. The host must allow
 * another explicit conversion and an escape to registration if the server
 * fails. Reset preserves this host policy; permission remains independently
 * per-key. */
uint8_t tgskk_session_defer_missing(TgskkSession *session, uint8_t enabled);
/* Last action queued an unfinished missing-candidate lookup (wait=0 hosts).
 * This does not poll, queue, or wait for network work. */
uint8_t tgskk_session_lookup_pending(const TgskkSession *session);
const TgskkView *tgskk_session_view(const TgskkSession *session);
const char *tgskk_session_error(const TgskkSession *session);
const char *tgskk_candidate_text(const TgskkSession *session, size_t index);
const char *tgskk_candidate_annotation(const TgskkSession *session,
                                       size_t index);

#ifdef __cplusplus
}
#endif
#endif
