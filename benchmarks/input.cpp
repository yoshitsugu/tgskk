#include "tgskk.h"
#include <algorithm>
#include <cctype>
#include <chrono>
#include <iostream>
#include <memory>
#include <stdexcept>
#include <string>
#include <thread>
#include <utility>
#include <vector>

using Clock = std::chrono::steady_clock;
using Session = std::unique_ptr<TgskkSession, decltype(&tgskk_session_free)>;

static void require(bool valid, const char *message) {
  if (!valid)
    throw std::runtime_error(message);
}

static double percentile(std::vector<double> values, size_t percent) {
  require(!values.empty(), "No latency samples");
  std::sort(values.begin(), values.end());
  return values[((values.size() - 1) * percent + 99) / 100];
}

struct Trace {
  const char *name;
  const char *keys;
  size_t candidates;
};

struct Samples {
  std::vector<double> keys, space, commit;
};

// The performance job also builds this file against the previous revision's
// header, so accept both context signatures.
static inline uint8_t setContext(uint8_t (*context)(TgskkSession *,
                                                    const char *, const char *,
                                                    uint8_t),
                                 TgskkSession *session) {
  return context(session, "駅で", "", 1);
}
static inline uint8_t setContext(uint8_t (*context)(TgskkSession *,
                                                    const char *, uint8_t),
                                 TgskkSession *session) {
  return context(session, "駅で", 1);
}

static void run(TgskkSession *session, const Trace &trace, Samples *samples) {
  for (const unsigned char key : std::string(trace.keys)) {
    const auto start = Clock::now();
    require(setContext(tgskk_session_context, session), "Context rejected");
    require(tgskk_session_network_allowed(session, 1),
            "Server permission rejected");
    const auto *view =
        tgskk_session_key(session, key, std::isupper(key) ? 1 : 0, 0);
    const double elapsed =
        std::chrono::duration<double, std::micro>(Clock::now() - start).count();
    require(view != nullptr && view->consumed, "Trace key was not consumed");
    if (samples)
      (key == ' ' ? samples->space : samples->keys).push_back(elapsed);
    if (key == ' ' && trace.candidates) {
      require(view->selecting && view->candidate_count == trace.candidates,
              "Conversion workload did not produce the expected candidates");
    }
  }
  const auto start = Clock::now();
  const auto *view = tgskk_session_key(session, 0xff0d, 0, 0);
  const double elapsed =
      std::chrono::duration<double, std::micro>(Clock::now() - start).count();
  require(view != nullptr, "Cannot confirm composition");
  if (samples)
    samples->commit.push_back(elapsed);
}

int main(int argc, char **argv) {
  try {
    require(argc == 5, "Usage: input DICTIONARY WORKER MODEL SERVER_PROBE");
    char *error = nullptr;
    std::unique_ptr<TgskkRuntime, decltype(&tgskk_runtime_free)> runtime(
        tgskk_runtime_new(argv[1], "utf-8", nullptr, &error),
        tgskk_runtime_free);
    if (error) {
      std::string message(error);
      tgskk_string_free(error);
      throw std::runtime_error(message);
    }
    require(runtime != nullptr, "Cannot open benchmark dictionary");
    std::unique_ptr<TgskkZenz, decltype(&tgskk_zenz_free)> ai(
        tgskk_zenz_new(argv[2], argv[3], &error), tgskk_zenz_free);
    if (error) {
      std::string message(error);
      tgskk_string_free(error);
      throw std::runtime_error(message);
    }
    require(ai != nullptr, "Cannot start AI worker");
    const auto deadline = Clock::now() + std::chrono::seconds(15);
    while (tgskk_zenz_status(ai.get()) == 0 && Clock::now() < deadline)
      std::this_thread::sleep_for(std::chrono::milliseconds(1));
    if (tgskk_zenz_status(ai.get()) != 1) {
      char *detail = tgskk_zenz_take_error(ai.get());
      std::string message = detail ? detail : "AI worker did not become ready";
      tgskk_string_free(detail);
      throw std::runtime_error(message);
    }

    std::unique_ptr<TgskkSkkserv, decltype(&tgskk_skkserv_free)> server(
        tgskk_skkserv_google_new(argv[4], 30, &error), tgskk_skkserv_free);
    require(server != nullptr && error == nullptr,
            "Cannot configure server probe");

    const Trace traces[] = {{"kana", "nihongonyuuryoku", 0},
                            {"conversion", "Kisha ", 4},
                            {"many_candidates", "Kou ", 512}};
    std::cout << '{';
    bool first = true;
    for (bool assisted : {false, true}) {
      for (const auto &trace : traces) {
        Session session(tgskk_session_new(runtime.get(), &error),
                        tgskk_session_free);
        require(session != nullptr && error == nullptr,
                "Cannot create session");
        require(tgskk_session_set_skkserv(session.get(), server.get()),
                "Cannot enable SKK server");
        if (assisted)
          require(tgskk_session_set_zenz(session.get(), ai.get()),
                  "Cannot enable AI");
        for (int i = 0; i < 200; ++i)
          run(session.get(), trace, nullptr);
        Samples samples;
        samples.keys.reserve(15000);
        samples.space.reserve(1000);
        samples.commit.reserve(1000);
        for (int i = 0; i < 1000; ++i)
          run(session.get(), trace, &samples);
        const std::pair<const char *, const std::vector<double> *>
            categories[] = {{"keys", &samples.keys},
                            {"space", &samples.space},
                            {"commit", &samples.commit}};
        for (const auto &category : categories) {
          if (category.second->empty())
            continue;
          for (size_t percent : {50, 99}) {
            if (!first)
              std::cout << ',';
            first = false;
            std::cout << '"' << "input_" << (assisted ? "ai_" : "plain_")
                      << trace.name << '_' << category.first << "_p" << percent
                      << "_us\":" << percentile(*category.second, percent);
          }
        }
      }
    }
    std::cout << "}\n";
    require(tgskk_zenz_status(ai.get()) == 1,
            "AI became unavailable during input");
    char *detail = tgskk_zenz_take_error(ai.get());
    if (detail) {
      std::string message(detail);
      tgskk_string_free(detail);
      throw std::runtime_error(message);
    }
    return 0;
  } catch (const std::exception &error) {
    std::cerr << error.what() << '\n';
    return 1;
  }
}
