#include "tgskk.h"
#include <cstdio>
#include <cstdlib>
#include <string>

static void require(bool value, const char *message) {
  if (!value) {
    std::fprintf(stderr, "%s\n", message);
    std::exit(1);
  }
}
int main(int argc, char **argv) {
  require(argc == 2, "Usage: core-test dictionary");
  char *error = nullptr;
  auto *runtime = tgskk_runtime_new(argv[1], "utf-8", nullptr, &error);
  if (!runtime) {
    std::fprintf(stderr, "%s\n", error ? error : "runtime failed");
    return 1;
  }
  auto *session = tgskk_session_new(runtime, &error);
  require(session != nullptr, "session failed");
  for (const unsigned char key : std::string("Kanji")) {
    for (unsigned i = 0; i < 3; ++i)
      require(tgskk_session_may_process(session, key, 0) == 1,
              "eligibility rejected kana");
    const auto *view = tgskk_session_key(session, key, 0, 0);
    require(view && view->consumed, "input was not consumed");
  }
  auto *view = tgskk_session_key(session, 0x20, 0, 0);
  require(view && view->candidate_count > 0, "no conversion candidates");
  require(std::string(tgskk_candidate_text(session, 0)) == "漢字",
          "wrong first candidate");
  view = tgskk_session_key(session, 0xff0d, 0, 0);
  require(view && std::string(view->committed) == "漢字", "wrong commit");
  view = tgskk_session_reset(session);
  require(view && std::string(view->preedit).empty(),
          "reset retained composition");
  tgskk_session_free(session);
  tgskk_runtime_free(runtime);
  tgskk_string_free(error);
  std::puts("Windows C ABI kana, repeated eligibility, candidates, commit and "
            "reset passed.");
}
