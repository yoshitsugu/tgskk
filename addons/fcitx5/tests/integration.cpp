#include <fcitx-utils/utf8.h>
// SPDX-License-Identifier: GPL-3.0-or-later
#include <fcitx-utils/capabilityflags.h>
#include <fcitx-utils/eventdispatcher.h>
#include <fcitx-utils/log.h>
#include <fcitx-utils/standardpaths.h>
#include <fcitx/addonfactory.h>
#include <fcitx/addonmanager.h>
#include <fcitx/candidatelist.h>
#include <fcitx/focusgroup.h>
#include <fcitx/inputcontext.h>
#include <fcitx/inputcontextmanager.h>
#include <fcitx/inputmethodengine.h>
#include <fcitx/inputmethodgroup.h>
#include <fcitx/inputmethodmanager.h>
#include <fcitx/inputpanel.h>
#include <fcitx/instance.h>

#include <cstdlib>
#include <chrono>
#include <thread>
#include <filesystem>
#include <fstream>
#include <sstream>
#include <string>
#include <vector>

using namespace fcitx;

// The installed fcitx5 executable embeds its keyboard addon. The isolated test
// instance needs a neutral first input method without launching that
// executable.
class TestKeyboard : public InputMethodEngine {
public:
  std::vector<InputMethodEntry> listInputMethods() override {
    std::vector<InputMethodEntry> entries;
    entries.emplace_back("keyboard-us", "Test keyboard", "en", "keyboard");
    return entries;
  }
  void keyEvent(const InputMethodEntry &, KeyEvent &) override {}
};

class TestKeyboardFactory : public AddonFactory {
  AddonInstance *create(AddonManager *) override { return new TestKeyboard; }
};

class TestContext : public InputContext {
public:
  explicit TestContext(InputContextManager &manager)
      : InputContext(manager, "tgskk-test") {
    setCapabilityFlags(CapabilityFlag::Preedit);
    created();
  }
  ~TestContext() override { destroy(); }
  const char *frontend() const override { return "tgskk-test"; }
  std::vector<std::string> commits;
  void commitStringImpl(const std::string &text) override {
    commits.push_back(text);
  }
  void deleteSurroundingTextImpl(int, unsigned int) override {}
  void forwardKeyImpl(const ForwardKeyEvent &) override {}
  void updatePreeditImpl() override {}
};

bool send(TestContext &context, const std::string &name, bool release = false) {
  Key key(name);
  if (name.size() == 1 && name[0] >= 'A' && name[0] <= 'Z') {
    key = Key(key.sym(), KeyState::Shift);
  }
  KeyEvent event(&context, key, release);
  return context.keyEvent(event);
}

void sequence(TestContext &context, const std::string &keys) {
  std::istringstream stream(keys);
  for (std::string key; stream >> key;) {
    send(context, key);
  }
}

void verifyCandidateOwnershipAndFocus(Instance &instance, FocusGroup &focus) {
  TestContext first(instance.inputContextManager()),
      second(instance.inputContextManager());
  first.setFocusGroup(&focus);
  second.setFocusGroup(&focus);
  first.focusIn();
  instance.setCurrentInputMethod(&first, "tgskk", false);
  sequence(first, "K a n j i space");
  auto oldList = first.inputPanel().candidateList();
  const auto firstCommits = first.commits.size();
  bool checkedUnfocused = false;
  auto watcher = instance.watchEvent(
      EventType::InputContextFocusOut, EventWatcherPhase::PreInputMethod,
      [&](Event &event) {
        if (static_cast<InputContextEvent &>(event).inputContext() == &first) {
          FCITX_ASSERT(!first.hasFocus());
          oldList->candidate(0).select(&first);
          FCITX_ASSERT(first.commits.size() == firstCommits);
          checkedUnfocused = true;
        }
      });
  second.focusIn();
  FCITX_ASSERT(checkedUnfocused);
  instance.setCurrentInputMethod(&second, "tgskk", false);
  sequence(second, "K i s h a space");
  const auto secondCommits = second.commits.size();
  oldList->candidate(2).select(&second);
  FCITX_ASSERT(second.commits.size() == secondCommits);
  FCITX_ASSERT(second.inputPanel().clientPreedit().toString() == "▼記者");
}

void verifyResetModeNotification(Instance &instance, InputMethodEngine &engine,
                                 FocusGroup &focus) {
  TestContext context(instance.inputContextManager());
  context.setFocusGroup(&focus);
  context.focusIn();
  instance.setCurrentInputMethod(&context, "tgskk", false);
  sequence(context, "q T o u k y o u space a");
  InputMethodEntry entry("tgskk", "tgskk", "ja", "tgskk");
  FCITX_ASSERT(engine.subModeLabel(entry, context) == "あ");
  int statusUpdates = 0;
  auto watcher = instance.watchEvent(
      EventType::InputContextUpdateUI, EventWatcherPhase::PreInputMethod,
      [&](Event &event) {
        auto &update = static_cast<InputContextUpdateUIEvent &>(event);
        if (update.inputContext() == &context &&
            update.component() == UserInterfaceComponent::StatusArea) {
          ++statusUpdates;
        }
      });
  context.reset();
  FCITX_ASSERT(statusUpdates > 0);
  FCITX_ASSERT(engine.subModeLabel(entry, context) == "ア");
  FCITX_ASSERT(context.inputPanel().clientPreedit().empty());
}

void verifyDelayedCandidates(Instance &instance, InputMethodEngine &engine,
                             FocusGroup &focus) {
  RawConfig defaults;
  engine.getConfig()->save(defaults);
  FCITX_ASSERT(*defaults.valueByPath("InlineCandidates") == "4");
  RawConfig config = defaults;
  config.setValueByPath("ZenzEnabled", "False");
  engine.setConfig(config);
  TestContext context(instance.inputContextManager());
  context.setFocusGroup(&focus);
  context.focusIn();
  instance.setCurrentInputMethod(&context, "tgskk", false);
  sequence(context, "I c h i r a n");
  for (const auto &text : {"一覧一", "一覧二", "一覧三", "一覧四"}) {
    send(context, "space");
    FCITX_ASSERT(context.inputPanel().clientPreedit().toString() == "▼" + std::string(text));
    FCITX_ASSERT(!context.inputPanel().candidateList());
    FCITX_ASSERT(context.inputPanel().preedit().empty());
    FCITX_ASSERT(context.inputPanel().auxUp().empty());
    if (std::string(text) == "一覧一") {
      FCITX_ASSERT(send(context, "Down"));
      FCITX_ASSERT(context.inputPanel().clientPreedit().toString() == "▼一覧二");
      FCITX_ASSERT(!context.inputPanel().candidateList());
      sequence(context, "Up Up");
      FCITX_ASSERT(context.inputPanel().clientPreedit().toString() == "▼一覧一");
    }
  }
  send(context, "Down");
  FCITX_ASSERT(context.inputPanel().candidateList());
  send(context, "Up");
  FCITX_ASSERT(!context.inputPanel().candidateList());
  FCITX_ASSERT(context.inputPanel().clientPreedit().toString() == "▼一覧四");
  send(context, "x");
  FCITX_ASSERT(context.inputPanel().clientPreedit().toString() == "▼一覧三");
  FCITX_ASSERT(!context.inputPanel().candidateList());
  send(context, "space");
  FCITX_ASSERT(!context.inputPanel().candidateList());
  send(context, "space");
  auto list = context.inputPanel().candidateList();
  FCITX_ASSERT(list && list->candidate(0).text().toString() == "一覧五");
  FCITX_ASSERT(context.inputPanel().clientPreedit().toString() == "▼一覧五");
  send(context, "space");
  FCITX_ASSERT(context.inputPanel().clientPreedit().toString() == "▼一覧十二");
  send(context, "Page_Down");
  FCITX_ASSERT(context.inputPanel().clientPreedit().toString() == "▼一覧十二");
  send(context, "Page_Up");
  FCITX_ASSERT(context.inputPanel().clientPreedit().toString() == "▼一覧五");
  send(context, "Page_Down");
  FCITX_ASSERT(context.inputPanel().clientPreedit().toString() == "▼一覧十二");
  send(context, "space");
  FCITX_ASSERT(context.inputPanel().clientPreedit().toString().find("【") != std::string::npos);
  send(context, "Control+g");
  FCITX_ASSERT(context.inputPanel().clientPreedit().toString() == "▼一覧十五");
  FCITX_ASSERT(context.inputPanel().candidateList());
  send(context, "x");
  FCITX_ASSERT(context.inputPanel().clientPreedit().toString() == "▼一覧五");
  send(context, "x");
  FCITX_ASSERT(context.inputPanel().clientPreedit().toString() == "▼一覧四");
  FCITX_ASSERT(!context.inputPanel().candidateList());
  list->candidate(0).select(&context);
  FCITX_ASSERT(context.commits.empty());
  send(context, "space");
  sequence(context, "Down Up");
  FCITX_ASSERT(context.inputPanel().clientPreedit().toString() == "▼一覧五");
  send(context, "2");
  FCITX_ASSERT(context.commits.back() == "一覧六");
  FCITX_ASSERT(!context.inputPanel().candidateList());
  const auto committed = context.commits.size();
  RawConfig temporary = config;
  temporary.setValueByPath("InlineCandidates", "3");
  engine.setConfig(temporary);
  FCITX_ASSERT(context.commits.size() == committed);
  engine.setConfig(config);
  FCITX_ASSERT(context.commits.size() == committed);

  sequence(context, "K u r i k k u space");
  const auto inlineWord = context.inputPanel().clientPreedit().toString();
  FCITX_ASSERT(send(context, "Page_Up"));
  FCITX_ASSERT(context.inputPanel().clientPreedit().toString() == inlineWord);
  FCITX_ASSERT(send(context, "Page_Down"));
  list = context.inputPanel().candidateList();
  FCITX_ASSERT(list && list->candidate(1).text().toString() == "クリック六");
  list->candidate(1).select(&context);
  FCITX_ASSERT(context.commits.back() == "クリック六");

  sequence(context, "I c h i r a n space space space space space");
  auto paged = std::dynamic_pointer_cast<CommonCandidateList>(context.inputPanel().candidateList());
  FCITX_ASSERT(paged);
  paged->next();
  const auto scrolled = paged->candidate(paged->cursorIndex()).text().toString();
  send(context, "Return");
  FCITX_ASSERT(context.commits.back() == scrolled);

  sequence(context, "I c h i r a n");
  const auto preedit = context.inputPanel().clientPreedit().toString();
  config.setValueByPath("InlineCandidates", "3");
  engine.setConfig(config);
  FCITX_ASSERT(context.inputPanel().clientPreedit().toString() == preedit);
  sequence(context, "space space space");
  FCITX_ASSERT(!context.inputPanel().candidateList());
  send(context, "space");
  FCITX_ASSERT(context.inputPanel().candidateList());
  const auto selected = context.inputPanel().clientPreedit().toString();
  config.setValueByPath("InlineCandidates", "0");
  engine.setConfig(config);
  auto full = std::dynamic_pointer_cast<CommonCandidateList>(context.inputPanel().candidateList());
  FCITX_ASSERT(full && full->totalSize() == 15);
  FCITX_ASSERT(full->globalCursorIndex() == 3);
  FCITX_ASSERT(context.inputPanel().clientPreedit().toString() == selected);
  config.setValueByPath("InlineCandidates", "100");
  engine.setConfig(config);
  FCITX_ASSERT(!context.inputPanel().candidateList());
  FCITX_ASSERT(context.inputPanel().clientPreedit().toString() == selected);
  const auto commits = context.commits.size();
  full->candidate(0).select(&context);
  FCITX_ASSERT(context.commits.size() == commits);
  RawConfig invalid = config;
  invalid.setValueByPath("InlineCandidates", "-1");
  engine.setConfig(invalid);
  RawConfig after;
  engine.getConfig()->save(after);
  FCITX_ASSERT(*after.valueByPath("InlineCandidates") == "4");
  context.reset();

  sequence(context, "K a n j i space");
  FCITX_ASSERT(send(context, "Page_Up"));
  FCITX_ASSERT(context.inputPanel().clientPreedit().toString() == "▼漢字");
  FCITX_ASSERT(send(context, "Page_Down"));
  FCITX_ASSERT(context.inputPanel().clientPreedit().toString() == "▼幹事");
  FCITX_ASSERT(!context.inputPanel().candidateList());
  FCITX_ASSERT(send(context, "Page_Down"));
  FCITX_ASSERT(context.inputPanel().clientPreedit().toString() == "▼幹事");
  send(context, "space");
  FCITX_ASSERT(context.inputPanel().clientPreedit().toString().find("【") != std::string::npos);
  send(context, "Control+g");
  FCITX_ASSERT(context.inputPanel().clientPreedit().toString() == "▼幹事");
  FCITX_ASSERT(!context.inputPanel().candidateList());
  context.reset();
  config.setValueByPath("InlineCandidates", "0");
  engine.setConfig(config);
  sequence(context, "K a n j i space x");
  FCITX_ASSERT(context.inputPanel().clientPreedit().toString() == "▽かんじ");
  FCITX_ASSERT(!context.inputPanel().candidateList());
  context.reset();
  context.focusOut();
  engine.setConfig(defaults);
}

void verifyZenz(Instance &instance);

void verify(Instance &instance) {
  auto group = instance.inputMethodManager().currentGroup();
  group.inputMethodList().clear();
  group.inputMethodList().emplace_back("keyboard-us");
  group.inputMethodList().emplace_back("tgskk");
  group.setDefaultInputMethod("tgskk");
  instance.inputMethodManager().setGroup(std::move(group));
  auto *addon = instance.addonManager().addon("tgskk", true);
  FCITX_ASSERT(addon);
  auto *engine = dynamic_cast<InputMethodEngine *>(addon);
  FCITX_ASSERT(engine);

  FocusGroup focus("tgskk-test", instance.inputContextManager());
  verifyDelayedCandidates(instance, *engine, focus);
  RawConfig immediate;
  engine->getConfig()->save(immediate);
  immediate.setValueByPath("InlineCandidates", "0");
  engine->setConfig(immediate);
  verifyCandidateOwnershipAndFocus(instance, focus);
  verifyResetModeNotification(instance, *engine, focus);
  TestContext first(instance.inputContextManager()),
      second(instance.inputContextManager());
  first.setFocusGroup(&focus);
  second.setFocusGroup(&focus);
  first.focusIn();
  instance.setCurrentInputMethod(&first, "tgskk", false);
  FCITX_ASSERT(instance.inputMethod(&first) == "tgskk");
  sequence(first, "K a n j i space");
  FCITX_ASSERT(first.inputPanel().clientPreedit().toString() == "▼漢字");
  auto list = first.inputPanel().candidateList();
  FCITX_ASSERT(list && list->size() == 3)
      << "preedit=" << first.inputPanel().clientPreedit().toString()
      << " size=" << (list ? list->size() : -1);
  FCITX_ASSERT(list->candidate(1).text().toString() == "感じ");
  FCITX_ASSERT(!send(first, "a", true));
  FCITX_ASSERT(first.inputPanel().clientPreedit().toString() == "▼漢字");
  sequence(first, "Down Return");
  FCITX_ASSERT(first.commits == std::vector<std::string>{"感じ"});
  FCITX_ASSERT(first.inputPanel().clientPreedit().empty());
  FCITX_ASSERT(!first.inputPanel().candidateList());
  InputContextEvent activated(&first, EventType::InputContextFocusIn);
  engine->activate(InputMethodEntry("tgskk", "tgskk", "ja", "tgskk"),
                   activated);
  FCITX_ASSERT(first.commits.size() == 1);

  sequence(first, "K a n j i space");
  list = first.inputPanel().candidateList();
  const auto clicked = list->candidate(2).text().toString();
  list->candidate(2).select(&first);
  FCITX_ASSERT(first.commits.back() == clicked);

  for (const auto &keys :
       {"K a n j", "K a K", "K a n j i space", "T o u k y o u space a",
        "T o u k y o u space N a n i space"}) {
    first.reset();
    sequence(first, keys);
    const auto commitsBefore = first.commits.size();
    second.focusIn();
    instance.setCurrentInputMethod(&second, "tgskk", false);
    FCITX_ASSERT(first.inputPanel().clientPreedit().empty());
    FCITX_ASSERT(!first.inputPanel().candidateList());
    FCITX_ASSERT(first.commits.size() == commitsBefore);
    sequence(second, "k a");
    FCITX_ASSERT(second.commits.back() == "か");
    first.focusIn();
    sequence(first, "k a");
    FCITX_ASSERT(first.commits.back() == "か");
    FCITX_ASSERT(first.inputPanel().clientPreedit().empty());
  }

  sequence(first, "K a n j i space");
  auto stale = first.inputPanel().candidateList();
  first.reset();
  sequence(first, "K a n j i space");
  const auto commitsBefore = first.commits.size();
  stale->candidate(0).select(&first);
  FCITX_ASSERT(first.commits.size() == commitsBefore);
  const auto nextChoice =
      first.inputPanel().candidateList()->candidate(1).text().toString();
  sequence(first, "Down Return");
  FCITX_ASSERT(first.commits.size() == commitsBefore + 1);
  FCITX_ASSERT(first.commits.back() == nextChoice);

  sequence(first, "T e s u t o space Page_Down");
  list = first.inputPanel().candidateList();
  FCITX_ASSERT(list && list->candidate(0).text().toString() == "候補八");
  const auto pageChoice = list->candidate(0).text().toString();
  sequence(first, "1");
  FCITX_ASSERT(first.commits.back() == pageChoice);

  sequence(first, "T e s u t o space");
  auto paged = std::dynamic_pointer_cast<CommonCandidateList>(
      first.inputPanel().candidateList());
  FCITX_ASSERT(paged);
  paged->next();
  const auto scrollChoice =
      paged->candidate(paged->cursorIndex()).text().toString();
  sequence(first, "Return");
  FCITX_ASSERT(first.commits.back() == scrollChoice);

  sequence(first, "T e s u t o space Up");
  FCITX_ASSERT(first.inputPanel().candidateList());
  auto bounds = std::dynamic_pointer_cast<CommonCandidateList>(
      first.inputPanel().candidateList());
  FCITX_ASSERT(bounds);
  const auto lastChoice =
      bounds->candidateFromAll(bounds->totalSize() - 1).text().toString();
  for (int i = 0; i < 12; ++i) {
    send(first, "Down");
  }
  FCITX_ASSERT(first.inputPanel().candidateList());
  FCITX_ASSERT(first.inputPanel().clientPreedit().toString() ==
               "▼" + lastChoice);
  FCITX_ASSERT(first.inputPanel().clientPreedit().toString().find("【") ==
               std::string::npos);
  sequence(first, "space");
  FCITX_ASSERT(first.inputPanel().clientPreedit().toString().find("【") !=
               std::string::npos);
  first.reset();

  first.setCapabilityFlags(CapabilityFlag::NoFlag);
  sequence(first, "K a n j i space");
  FCITX_ASSERT(first.inputPanel().preedit().toString().starts_with("▼"));
  first.reset();
  first.setCapabilityFlags(CapabilityFlag::Preedit);

  sequence(first, "K a n j i space");
  instance.setCurrentInputMethod(&first, "keyboard-us", false);
  FCITX_ASSERT(instance.inputMethod(&first) == "keyboard-us");
  instance.setCurrentInputMethod(&first, "tgskk", false);
  FCITX_ASSERT(first.inputPanel().clientPreedit().empty());
  sequence(first, "q");
  FCITX_ASSERT(
      engine->subModeLabel(InputMethodEntry("tgskk", "tgskk", "ja", "tgskk"),
                           first) == "ア");
  FCITX_ASSERT(first.inputPanel().overlayMessage().toString() == "ア");
  sequence(first, "a");
  FCITX_ASSERT(!send(first, "Control+c"));
  sequence(first, "K a n j");
  second.focusIn();
  first.focusIn();
  sequence(first, "k a");
  FCITX_ASSERT(first.commits.back() == "カ");
  sequence(first, "K a n");
  const auto beforeConfig = first.inputPanel().clientPreedit().toString();
  RawConfig before;
  engine->getConfig()->save(before);
  RawConfig invalid;
  engine->getConfig()->save(invalid);
  invalid.setValueByPath("Dictionary", "/nonexistent/tgskk-dictionary");
  engine->setConfig(invalid);
  FCITX_ASSERT(first.inputPanel().clientPreedit().toString() == beforeConfig);
  RawConfig after;
  engine->getConfig()->save(after);
  FCITX_ASSERT(after.valueByPath("Dictionary") &&
               before.valueByPath("Dictionary"));
  FCITX_ASSERT(*after.valueByPath("Dictionary") ==
               *before.valueByPath("Dictionary"));

  RawConfig indicator;
  engine->getConfig()->save(indicator);
  indicator.setValueByPath("ModeIndicator", "False");
  engine->setConfig(indicator);
  FCITX_ASSERT(first.inputPanel().clientPreedit().toString() == beforeConfig);
  first.reset();
  sequence(first, "q k a");
  FCITX_ASSERT(first.commits.back() == "か");

  auto selectText = [&](const std::string &text) {
    auto candidates = first.inputPanel().candidateList();
    FCITX_ASSERT(candidates);
    for (int index = 0; index < candidates->size(); ++index) {
      if (candidates->candidate(index).text().toString() == text) {
        candidates->candidate(index).select(&first);
        return;
      }
    }
    FCITX_ASSERT(false) << "Missing candidate: " << text;
  };
  sequence(first, "K a n j i space");
  selectText("漢字");
  engine->save();
  sequence(first, "K a n j i space");
  const std::string learned = "感じ";
  selectText(learned);
  const auto originalDictionary = *before.valueByPath("Dictionary");
  const auto replacementPath =
      std::filesystem::path(originalDictionary).parent_path() /
      "alternative.utf8";
  std::filesystem::copy_file(originalDictionary, replacementPath);
  RawConfig replacement;
  engine->getConfig()->save(replacement);
  replacement.setValueByPath("Dictionary", replacementPath.string());
  engine->setConfig(replacement);
  FCITX_ASSERT(
      engine->subModeLabel(InputMethodEntry("tgskk", "tgskk", "ja", "tgskk"),
                           first) == "あ");
  sequence(first, "k a");
  FCITX_ASSERT(first.commits.back() == "か");
  sequence(first, "K a n j i space");
  FCITX_ASSERT(first.inputPanel().clientPreedit().toString() == "▼" + learned);
  auto afterReplacement = first.inputPanel().candidateList();
  const auto recovered = afterReplacement->candidate(1).text().toString();
  afterReplacement->candidate(1).select(&first);
  const auto userPath =
      StandardPaths::global().userDirectory(StandardPathsType::PkgData) /
      "tgskk/SKK-JISYO.user";
  FCITX_ASSERT(std::filesystem::remove(userPath));
  engine->save();
  FCITX_ASSERT(std::filesystem::exists(userPath));
  replacement.setValueByPath("Dictionary", originalDictionary);
  replacement.setValueByPath("ModeIndicator", "True");
  engine->setConfig(replacement);
  sequence(first, "K a n j i space");
  FCITX_ASSERT(first.inputPanel().clientPreedit().toString() ==
               "▼" + recovered);
  first.reset();

  sequence(first, "E r a minus space");
  FCITX_ASSERT(
      engine->subModeLabel(InputMethodEntry("tgskk", "tgskk", "ja", "tgskk"),
                           first) == "!");
  FCITX_ASSERT(first.inputPanel().clientPreedit().empty());
  sequence(first, "k a");
  FCITX_ASSERT(first.commits.back() == "か");
  FCITX_ASSERT(
      engine->subModeLabel(InputMethodEntry("tgskk", "tgskk", "ja", "tgskk"),
                           first) == "あ");
  RawConfig savedZenz;
  engine->getConfig()->save(savedZenz);
  RawConfig missingZenz = savedZenz;
  missingZenz.setValueByPath("ZenzEnabled", "True");
  missingZenz.setValueByPath("ZenzModel", "/nonexistent/tgskk-test-model");
  engine->setConfig(missingZenz);
  std::this_thread::sleep_for(std::chrono::milliseconds(80));
  sequence(first, "k a");
  FCITX_ASSERT(first.inputPanel().auxUp().empty());
  RawConfig missingQuiet = missingZenz;
  missingQuiet.setValueByPath("InlineCandidates", "4");
  engine->setConfig(missingQuiet);
  sequence(first, "K a n j i space");
  FCITX_ASSERT(!first.inputPanel().candidateList());
  FCITX_ASSERT(first.inputPanel().auxUp().empty());
  first.reset();
  engine->setConfig(missingZenz);
  sequence(first, "l a b c");
  FCITX_ASSERT(first.inputPanel().auxUp().empty());
  sequence(first, "Control+j K a n j i space");
  FCITX_ASSERT(first.inputPanel().auxUp().toString().find("Zenz unavailable") != std::string::npos);
  send(first, "Return");
  FCITX_ASSERT(first.inputPanel().auxUp().empty());
  engine->setConfig(savedZenz);
  engine->save();
  first.focusOut();
  FCITX_ASSERT(first.inputPanel().empty());
  verifyZenz(instance);
}

void verifyZenz(Instance &instance) {
  if (!std::getenv("TGSKK_TEST_ZENZ")) { return; }
  FocusGroup focus("zenz-test", instance.inputContextManager());
  TestContext context(instance.inputContextManager());
  context.setFocusGroup(&focus);
  context.focusIn();
  instance.setCurrentInputMethod(&context, "tgskk", false);
  std::this_thread::sleep_for(std::chrono::milliseconds(150));
  auto convert = [&](const std::string &left, bool wait) {
    context.surroundingText().setText(left, utf8::length(left), utf8::length(left));
    sequence(context, "K i s h a");
    if (wait) { std::this_thread::sleep_for(std::chrono::milliseconds(120)); }
    send(context, "space");
    return context.inputPanel().clientPreedit().toString();
  };
  const auto station = convert("駅で", true);
  FCITX_ASSERT(station == "▼汽車") << "preedit=" << station << " aux=" << context.inputPanel().auxUp().toString();
  FCITX_ASSERT(context.inputPanel().auxUp().toString() == "AI-ranked");
  context.reset();
  FCITX_ASSERT(convert("新聞の", true) == "▼記者");
  context.reset();
  context.setCapabilityFlags(CapabilityFlags{CapabilityFlag::Preedit, CapabilityFlag::Sensitive});
  FCITX_ASSERT(convert("駅で", true) == "▼記者");
  FCITX_ASSERT(context.inputPanel().auxUp().empty());
  context.reset();
  context.setCapabilityFlags(CapabilityFlag::Preedit);
  context.surroundingText().setText("駅で秘密", 2, 4);
  sequence(context, "K i s h a");
  std::this_thread::sleep_for(std::chrono::milliseconds(120));
  send(context, "space");
  FCITX_ASSERT(context.inputPanel().clientPreedit().toString() == "▼記者");
  context.reset();
  // Without usable surrounding text, locally committed input supplies context.
  context.surroundingText().invalidate();
  sequence(context, "E k i space Return d e");
  sequence(context, "K i s h a");
  std::this_thread::sleep_for(std::chrono::milliseconds(120));
  send(context, "space");
  FCITX_ASSERT(context.inputPanel().clientPreedit().toString() == "▼汽車");
  context.reset();
  context.focusOut();
  context.focusIn();
  sequence(context, "K i s h a");
  std::this_thread::sleep_for(std::chrono::milliseconds(120));
  send(context, "space");
  FCITX_ASSERT(context.inputPanel().clientPreedit().toString() == "▼記者");
  context.reset();
  auto currentKisha = [&]() {
    sequence(context, "K i s h a");
    std::this_thread::sleep_for(std::chrono::milliseconds(120));
    send(context, "space");
    return context.inputPanel().clientPreedit().toString();
  };
  context.surroundingText().setText("大学教授の", 5, 5);
  sequence(context, "K o u e n");
  std::this_thread::sleep_for(std::chrono::milliseconds(120));
  send(context, "space");
  FCITX_ASSERT(context.inputPanel().clientPreedit().toString() == "▼講演");
  context.reset();

  // The application keeps reporting its pre-commit snapshot.
  context.surroundingText().setText("", 0, 0);
  sequence(context, "E k i space Return d e");
  FCITX_ASSERT(currentKisha() == "▼汽車");
  context.reset();

  context.surroundingText().invalidate();
  sequence(context, "E k i space Return d e");
  FCITX_ASSERT(!send(context, "Left"));
  FCITX_ASSERT(currentKisha() == "▼記者");
  context.reset();

  context.surroundingText().setText("駅で", 2, 2);
  send(context, "Control+j");
  context.surroundingText().invalidate();
  FCITX_ASSERT(currentKisha() == "▼記者");
  context.reset();

  context.setCapabilityFlags(CapabilityFlags{CapabilityFlag::Preedit, CapabilityFlag::Sensitive});
  sequence(context, "E k i d e space Return");
  FCITX_ASSERT(context.commits.back() == "駅で");
  context.setCapabilityFlags(CapabilityFlag::Preedit);
  FCITX_ASSERT(currentKisha() == "▼記者");
  context.reset();

  // A mouse selection can arrive after the application has changed its text.
  context.surroundingText().setText("新聞の", 3, 3);
  sequence(context, "E k i space");
  context.surroundingText().setText("", 0, 0);
  context.inputPanel().candidateList()->candidate(0).select(&context);
  FCITX_ASSERT(context.commits.back() == "駅");
  sequence(context, "d e");
  FCITX_ASSERT(currentKisha() == "▼汽車");
  context.reset();

  FCITX_ASSERT(convert("駅で", true) == "▼汽車");
  context.surroundingText().setText("更新された文脈", 7, 7);
  send(context, "Down");
  FCITX_ASSERT(context.inputPanel().auxUp().toString() == "AI-ranked");
  context.reset();

  // Once displayed, a candidate list stays fixed even if scoring completes.
  const auto first = convert("駅で", false);
  std::this_thread::sleep_for(std::chrono::milliseconds(120));
  FCITX_ASSERT(context.inputPanel().clientPreedit().toString() == first);
  context.reset();
  auto *engine = dynamic_cast<InputMethodEngine *>(instance.addonManager().addon("tgskk", true));
  FCITX_ASSERT(engine);
  RawConfig original;
  engine->getConfig()->save(original);
  RawConfig quiet = original;
  quiet.setValueByPath("InlineCandidates", "4");
  engine->setConfig(quiet);
  FCITX_ASSERT(convert("駅で", true) == "▼汽車");
  FCITX_ASSERT(!context.inputPanel().candidateList());
  FCITX_ASSERT(context.inputPanel().auxUp().empty());
  context.reset();
  context.focusOut();
  engine->setConfig(original);
}

int main() {
  char path[] = "/tmp/tgskk-fcitx5-test-XXXXXX";
  FCITX_ASSERT(mkdtemp(path));
  const std::filesystem::path root(path);
  setenv("SKIP_FCITX_PATH", "1", 1);
  setenv("FCITX_ADDON_DIRS",
         (std::string(TGSKK_BUILD_DIR) + ":" +
          StandardPaths::fcitxPath("addondir").string())
             .c_str(),
         1);
  setenv("FCITX_DATA_DIRS",
         (std::string(TGSKK_BUILD_DIR) +
          "/test-data:" + StandardPaths::fcitxPath("pkgdatadir").string())
             .c_str(),
         1);
  setenv("FCITX_CONFIG_HOME", (root / "config").c_str(), 1);
  setenv("FCITX_DATA_HOME", (root / "data").c_str(), 1);
  std::filesystem::create_directories(root / "config/conf");
  std::ofstream dictionary(root / "SKK-JISYO.utf8");
  std::ifstream demo(TGSKK_SOURCE_DIR "/examples/SKK-JISYO.demo.utf8");
  dictionary << demo.rdbuf();
  dictionary << "\nきしゃ /記者/汽車/帰社/貴社/\nえき /駅/\nえきで /駅で/\nこうえん /公園/講演/公演/後援/\n";
  dictionary << "\nてすと "
                "/候補一/候補二/候補三/候補四/候補五/候補六/候補七/候補八/"
                "候補九/候補十/\n";
  dictionary << "いちらん /一覧一/一覧二/一覧三/一覧四/一覧五/一覧六/一覧七/一覧八/一覧九/一覧十/一覧十一/一覧十二/一覧十三/一覧十四/一覧十五/\n";
  dictionary << "くりっく /クリック一/クリック二/クリック三/クリック四/クリック五/クリック六/クリック七/クリック八/\n";
  const char unrepresentable[] = "えらー /bad\0candidate/\n";
  dictionary.write(unrepresentable, sizeof(unrepresentable) - 1);
  dictionary.close();
  std::ofstream config(root / "config/conf/tgskk.conf");
  config << "Dictionary=" << (root / "SKK-JISYO.utf8").string()
         << "\nEncoding=utf-8\nModeIndicator=True\n";
  config.close();
  {
    char name[] = "test-tgskk", disable[] = "--disable=all",
         enable[] = "--enable=tgskk,keyboard";
    char *argv[]{name, disable, enable};
    TestKeyboardFactory keyboard;
    StaticAddonRegistry registry{{"keyboard", &keyboard}};
    Instance instance(3, argv);
    instance.addonManager().registerDefaultLoader(&registry);
    instance.eventDispatcher().schedule([&instance]() {
      verify(instance);
      instance.exit();
    });
    instance.exec();
  }
  std::filesystem::remove_all(root);
}
