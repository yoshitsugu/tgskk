// SPDX-License-Identifier: GPL-3.0-or-later
#include "../broker_queue.h"
#include <cstdio>
#include <stdexcept>
using namespace tgskk::windows;
static void require(bool value, const char *message) {
  if (!value)
    throw std::runtime_error(message);
}
static std::shared_ptr<Client> client() {
  return std::make_shared<Client>(
      Handle(CreateEventW(nullptr, TRUE, FALSE, nullptr)));
}
int main() {
  try {
    Queue queue;
    auto first = client(), second = client();
    Request key;
    key.op = Op::Key;
    key.argument = 'a';
    key.sequence = 12;
    require(queue.submit({first, key}), "queue initial key");
    {
      ReloadPause pause(queue);
      require(queue.jobs.empty() && first->ready,
              "queued key was left waiting for dictionary I/O");
      auto deferred = snapshot(first->reply);
      require(deferred.sequence == 12 && deferred.epoch == queue.epoch &&
                  deferred.failed && deferred.flags == reloadDeferred,
              "queued key lacked an explicit not-executed response");
      key.sequence = 13;
      require(queue.submit({second, key}) && second->ready &&
                  queue.jobs.empty(),
              "key arriving during reload was queued for execution");
      require(snapshot(second->reply).sequence == 13,
              "deferred key reply was mismatched");
    }
    require(queue.submit({first, key}) && queue.jobs.size() == 1,
            "queue did not resume after dictionary reload");
    for (uint32_t inlineCount : {0U, 2U, 7U}) {
      Snapshot page;
      page.epoch = 1;
      page.flags = 4;
      page.count = 16;
      page.selected = 8;
      page.start = static_cast<uint32_t>(
          tgskk::candidatePageStart(page.selected, inlineCount));
      page.candidates.resize(std::min<uint32_t>(7, page.count - page.start));
      require(snapshot(encode(page), inlineCount).start == page.start,
              "non-default inline count rejected a valid candidate page");
      bool rejected = false;
      try {
        (void)snapshot(encode(page), 4);
      } catch (const std::exception &) {
        rejected = true;
      }
      require(rejected, "wrong candidate page boundary was accepted");
    }
    std::puts("Reload queue handoff and configured candidate pages passed");
    return 0;
  } catch (const std::exception &error) {
    std::fprintf(stderr, "%s\n", error.what());
    return 1;
  }
}
