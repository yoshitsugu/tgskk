#include "engine.h"
#include <algorithm>
#include <atomic>
#include <filesystem>
#include <fstream>
#include <mutex>
#include <netinet/in.h>
#include <poll.h>
#include <string>
#include <sys/socket.h>
#include <thread>
#include <unistd.h>
#include <vector>

static bool failNextKey = false;
extern "C" const TgskkView *__real_tgskk_session_key(TgskkSession *, uint32_t,
                                                     uint32_t, uint8_t);
extern "C" const TgskkView *__wrap_tgskk_session_key(TgskkSession *session,
                                                     uint32_t key,
                                                     uint32_t modifiers,
                                                     uint8_t release) {
  if (failNextKey) {
    failNextKey = false;
    return nullptr;
  }
  return __real_tgskk_session_key(session, key, modifiers, release);
}

struct Server {
  int fd = socket(AF_INET, SOCK_STREAM | SOCK_CLOEXEC, 0);
  std::atomic<unsigned> requests{0};
  std::jthread thread;
  unsigned port;
  Server() {
    sockaddr_in address{};
    address.sin_family = AF_INET;
    address.sin_addr.s_addr = htonl(INADDR_LOOPBACK);
    g_assert_cmpint(
        bind(fd, reinterpret_cast<sockaddr *>(&address), sizeof(address)), ==,
        0);
    socklen_t length = sizeof(address);
    getsockname(fd, reinterpret_cast<sockaddr *>(&address), &length);
    port = ntohs(address.sin_port);
    g_assert_cmpint(listen(fd, 8), ==, 0);
    thread = std::jthread([this](std::stop_token stop) {
      while (!stop.stop_requested()) {
        pollfd event{fd, POLLIN, 0};
        if (poll(&event, 1, 20) <= 0) {
          continue;
        }
        int client = accept4(fd, nullptr, nullptr, SOCK_CLOEXEC);
        if (client < 0) {
          continue;
        }
        timeval timeout{1, 0};
        setsockopt(client, SOL_SOCKET, SO_RCVTIMEO, &timeout, sizeof(timeout));
        char byte;
        while (recv(client, &byte, 1, 0) == 1 && byte != ' ') {
        }
        ++requests;
        const std::string response = "1/外部/\n";
        send(client, response.data(), response.size(), MSG_NOSIGNAL);
        close(client);
      }
    });
  }
  ~Server() {
    thread.request_stop();
    thread.join();
    close(fd);
  }
};

struct Signal {
  std::string path, name;
  GVariant *body;
};
struct Capture {
  std::mutex mutex;
  std::vector<Signal> signals;
  ~Capture() {
    for (auto &s : signals) {
      g_variant_unref(s.body);
    }
  }
  GVariant *latest(const char *name, const char *path = "/test/one") {
    std::lock_guard lock(mutex);
    for (auto it = signals.rbegin(); it != signals.rend(); ++it) {
      if (it->name == name && it->path == path) {
        return it->body;
      }
    }
    g_error("Missing signal %s", name);
    return nullptr;
  }
  size_t commits() {
    std::lock_guard lock(mutex);
    return std::count_if(signals.begin(), signals.end(),
                         [](auto &s) { return s.name == "CommitText"; });
  }
};
static Capture capture;
static GDBusConnection *connection;
static void flush() {
  GError *error = nullptr;
  g_dbus_connection_flush_sync(connection, nullptr, &error);
  g_assert_no_error(error);
}
static IBusText *text(GVariant *body) {
  GVariant *value = g_variant_get_child_value(body, 0);
  auto *result = IBUS_TEXT(ibus_serializable_deserialize(value));
  g_variant_unref(value);
  return result;
}
static std::string committed() {
  flush();
  auto *value = text(capture.latest("CommitText"));
  std::string result(value->text);
  g_object_unref(value);
  return result;
}
static std::string preedit() {
  flush();
  auto *value = text(capture.latest("UpdatePreeditText"));
  std::string result(value->text);
  g_object_unref(value);
  return result;
}
static gboolean key(IBusEngine *engine, guint symbol, guint modifiers = 0) {
  return IBUS_ENGINE_GET_CLASS(engine)->process_key_event(engine, symbol, 0,
                                                          modifiers);
}
static void sequence(IBusEngine *engine, const char *ascii) {
  for (const char *p = ascii; *p; ++p) {
    key(engine, *p);
  }
}
int main() {
  ibus_init();
  g_log_set_always_fatal(
      static_cast<GLogLevelFlags>(G_LOG_LEVEL_ERROR | G_LOG_LEVEL_CRITICAL));
  Server server;
  GError *error = nullptr;
  gchar *directory = g_dir_make_tmp("tgskk-ibus-test-XXXXXX", &error);
  g_assert_no_error(error);
  std::string root(directory);
  g_free(directory);
  g_setenv("XDG_CONFIG_HOME", root.c_str(), TRUE);
  g_setenv("XDG_DATA_HOME", root.c_str(), TRUE);
  std::filesystem::create_directories(root + "/tgskk");
  {
    std::ofstream config(root + "/tgskk/ibus.conf");
    config << "[General]\nDictionary=" TGSKK_SOURCE_DIR "/examples/"
              "SKK-JISYO.demo.utf8\nDictionaryEncoding=utf-8\nZenzEnabled="
              "false\nSkkservMode=Disabled\n";
  }
  auto config = tgskk_ibus::Config::load();
  g_assert_true(config.userDictionary.starts_with(root + "/"));
  g_assert_cmpstr(config.encoding.c_str(), ==, "utf-8");
  {
    std::ofstream file(root + "/tgskk/ibus.conf");
    file << "[General]\nDictionaryEncoding=euc-jp\n";
  }
  const auto eucConfig = tgskk_ibus::Config::load();
  g_assert_cmpstr(eucConfig.encoding.c_str(), ==, "euc-jp");
  config.server = "Custom";
  config.host = "127.0.0.1";
  config.port = server.port;
  config.serverEncoding = "utf-8";
  config.dictionary = root + "/dictionary";
  std::filesystem::copy_file(TGSKK_SOURCE_DIR "/examples/SKK-JISYO.demo.utf8",
                             config.dictionary);
  {
    std::ofstream dictionary(config.dictionary, std::ios::app);
    dictionary << "いちらん "
                  "/一/二/三/四/五;注釈/六/七/八/九/十/十一/十二/十三/十四/"
                  "十五/十六/\n";
  }
  tgskk_ibus::resources = std::make_unique<tgskk_ibus::Resources>(config);
  connection = g_bus_get_sync(G_BUS_TYPE_SESSION, nullptr, &error);
  g_assert_no_error(error);
  const auto filter = g_dbus_connection_add_filter(
      connection,
      [](GDBusConnection *, GDBusMessage *message, gboolean incoming,
         gpointer) -> GDBusMessage * {
        if (!incoming &&
            g_dbus_message_get_message_type(message) ==
                G_DBUS_MESSAGE_TYPE_SIGNAL &&
            g_strcmp0(g_dbus_message_get_interface(message),
                      "org.freedesktop.IBus.Engine") == 0) {
          std::lock_guard lock(capture.mutex);
          capture.signals.push_back(
              {g_dbus_message_get_path(message),
               g_dbus_message_get_member(message),
               g_dbus_message_get_body(message)
                   ? g_variant_ref(g_dbus_message_get_body(message))
                   : g_variant_ref_sink(g_variant_new_tuple(nullptr, 0))});
        }
        return message;
      },
      nullptr, nullptr);
  auto make = [](const char *path) {
    return ibus_engine_new_with_type(tgskk_ibus::engineType(), "tgskk", path,
                                     connection);
  };
  auto *one = make("/test/one");
  auto *two = make("/test/two");
  auto *klass = IBUS_ENGINE_GET_CLASS(one);
  klass->focus_in(one);

  sequence(one, "Kanji ");
  g_assert_true(preedit() == "▼漢字");
  key(one, IBUS_KEY_Return);
  g_assert_true(committed() == "漢字");
  const auto count = capture.commits();
  klass->reset(one);
  flush();
  g_assert_cmpuint(capture.commits(), ==, count);
  sequence(one, "Ichiran ");
  g_assert_true(preedit() == "▼一");
  g_assert_true(key(one, IBUS_KEY_Page_Up));
  g_assert_true(preedit() == "▼一");
  klass->page_up(one);
  g_assert_true(preedit() == "▼一");
  sequence(one, "    ");
  g_assert_true(preedit() == "▼五");
  flush();
  GVariant *variant =
      g_variant_get_child_value(capture.latest("UpdateLookupTable"), 0);
  auto *table = IBUS_LOOKUP_TABLE(ibus_serializable_deserialize(variant));
  g_variant_unref(variant);
  g_assert_cmpstr(ibus_lookup_table_get_candidate(table, 0)->text, ==,
                  "五 注釈");
  g_object_unref(table);
  key(one, IBUS_KEY_space, IBUS_LOCK_MASK | IBUS_MOD2_MASK);
  g_assert_true(preedit() == "▼十二");
  key(one, IBUS_KEY_2);
  g_assert_true(committed() == "十三");
  sequence(one, "Ichiran ");
  key(one, IBUS_KEY_Page_Down);
  flush();
  variant = g_variant_get_child_value(capture.latest("UpdateLookupTable"), 0);
  table = IBUS_LOOKUP_TABLE(ibus_serializable_deserialize(variant));
  g_variant_unref(variant);
  std::string clicked = ibus_lookup_table_get_candidate(table, 1)->text;
  clicked = clicked.substr(0, clicked.find(' '));
  g_object_unref(table);
  klass->candidate_clicked(one, 1, 1, 0);
  g_assert_true(committed() == clicked);
  sequence(one, "Kanji");
  klass->focus_out(one);
  klass->focus_in(two);
  g_assert_false(key(two, IBUS_KEY_a, IBUS_RELEASE_MASK));
  key(two, IBUS_KEY_a);
  flush();
  auto *second = text(capture.latest("CommitText", "/test/two"));
  g_assert_cmpstr(second->text, ==, "あ");
  g_object_unref(second);
  klass->focus_in(one);
  g_assert_true(preedit() == "");
  failNextKey = true;
  g_assert_true(key(one, IBUS_KEY_K));
  sequence(one, "Kanji ");
  g_assert_true(preedit() == "▼漢字");
  klass->reset(one);
  auto *surrounding = ibus_text_new_from_string("駅で");
  g_object_ref_sink(surrounding);
  klass->set_surrounding_text(one, surrounding, 2, 2);
  klass->set_content_type(one, IBUS_INPUT_PURPOSE_PASSWORD, 0);
  sequence(one, "Kisha ");
  key(one, IBUS_KEY_Return);
  g_assert_true(committed() == "記者");
  g_assert_cmpuint(server.requests.load(), ==, 0);
  klass->reset(one);
  sequence(one, "Mikan ");
  g_assert_cmpuint(server.requests.load(), ==, 0);
  klass->reset(one);
  klass->set_content_type(one, IBUS_INPUT_PURPOSE_FREE_FORM, 0);
  klass->set_surrounding_text(one, surrounding, 2, 0);
  sequence(one, "Mikan ");
  g_assert_cmpuint(server.requests.load(), ==, 0);
  klass->reset(one);
  klass->set_surrounding_text(one, surrounding, 2, 2);
  sequence(one, "Mikan ");
  g_assert_true(preedit() == "▼外部");
  g_assert_cmpuint(server.requests.load(), ==, 1);
  klass->reset(one);
  klass->set_content_type(one, IBUS_INPUT_PURPOSE_FREE_FORM,
                          IBUS_INPUT_HINT_PRIVATE);
  sequence(one, "Mikan ");
  g_assert_true(preedit() != "▼外部");
  g_assert_cmpuint(server.requests.load(), ==, 1);
  klass->reset(one);
  klass->set_content_type(one, IBUS_INPUT_PURPOSE_FREE_FORM, 1u << 12);
  sequence(one, "Mikan ");
  g_assert_true(preedit() != "▼外部");
  g_assert_cmpuint(server.requests.load(), ==, 1);
  klass->reset(one);
  g_object_unref(surrounding);
  tgskk_ibus::resources->save();
  g_assert_true(std::filesystem::is_regular_file(config.userDictionary));
  ibus_object_destroy(IBUS_OBJECT(one));
  ibus_object_destroy(IBUS_OBJECT(two));
  g_object_unref(one);
  g_object_unref(two);
  flush();
  tgskk_ibus::resources.reset();
  if (const char *model = g_getenv("TGSKK_TEST_MODEL")) {
    config.ai = true;
    config.model = model;
    config.server = "Disabled";
    tgskk_ibus::resources = std::make_unique<tgskk_ibus::Resources>(config);
    auto *engine = make("/test/model");
    klass = IBUS_ENGINE_GET_CLASS(engine);
    klass->focus_in(engine);
    const auto deadline = g_get_monotonic_time() + 5000000;
    bool applied = false;
    while (g_get_monotonic_time() < deadline && !applied) {
      klass->reset(engine);
      auto *left = ibus_text_new_from_string("駅で");
      g_object_ref_sink(left);
      klass->set_surrounding_text(engine, left, 2, 2);
      g_object_unref(left);
      for (const char *p = "Kisha"; *p; ++p) {
        key(engine, *p);
        g_usleep(30000);
      }
      g_usleep(40000);
      key(engine, IBUS_KEY_space);
      flush();
      auto *shown = text(capture.latest("UpdatePreeditText", "/test/model"));
      applied = std::string(shown->text) == "▼汽車";
      g_object_unref(shown);
    }
    g_assert_true(applied);
    klass->reset(engine);
    klass->set_content_type(engine, IBUS_INPUT_PURPOSE_FREE_FORM,
                            IBUS_INPUT_HINT_PRIVATE);
    sequence(engine, "Kisha ");
    flush();
    auto *privateText =
        text(capture.latest("UpdatePreeditText", "/test/model"));
    g_assert_cmpstr(privateText->text, ==, "▼記者");
    g_object_unref(privateText);
    ibus_object_destroy(IBUS_OBJECT(engine));
    g_object_unref(engine);
    tgskk_ibus::resources.reset();
  }
  flush();
  g_dbus_connection_remove_filter(connection, filter);
  g_object_unref(connection);
  std::filesystem::remove_all(root);
}
