// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once
#include "broker_protocol.h"
#include <atomic>
#include <condition_variable>
#include <deque>
#include <mutex>
#include <utility>
namespace tgskk::windows {
struct Client {
  Channel channel;
  Handle canceled{CreateEventW(nullptr, TRUE, FALSE, nullptr)};
  std::atomic<bool> live = true;
  uint64_t next = 1;
  std::mutex mutex;
  std::condition_variable wake;
  bool ready = false;
  std::vector<uint8_t> reply;
  explicit Client(Handle pipe) : channel(std::move(pipe)) {}
};
struct Job {
  std::shared_ptr<Client> client;
  Request request;
};
struct Queue {
  std::mutex mutex;
  std::condition_variable wake;
  std::deque<Job> jobs;
  std::vector<std::weak_ptr<Client>> clients;
  std::atomic<unsigned> connections = 0;
  std::atomic<bool> stopped = false;
  uint64_t epoch = randomId();
  bool reloading = false;
  void defer(const Job &job) {
    Snapshot reply;
    reply.sequence = job.request.sequence;
    reply.epoch = epoch;
    reply.failed = true;
    reply.flags = reloadDeferred;
    auto bytes = encode(reply);
    std::lock_guard lock(job.client->mutex);
    job.client->reply = std::move(bytes);
    job.client->ready = true;
    job.client->wake.notify_one();
  }
  bool submit(Job job) {
    std::lock_guard lock(mutex);
    if (reloading)
      defer(job);
    else {
      if (jobs.size() >= 32)
        return false;
      jobs.push_back(std::move(job));
      wake.notify_one();
    }
    return true;
  }
};
class ReloadPause {
  Queue &queue;

public:
  explicit ReloadPause(Queue &value) : queue(value) {
    std::lock_guard lock(queue.mutex);
    while (!queue.jobs.empty()) {
      queue.defer(queue.jobs.front());
      queue.jobs.pop_front();
    }
    queue.reloading = true;
  }
  ~ReloadPause() {
    std::lock_guard lock(queue.mutex);
    queue.reloading = false;
  }
  ReloadPause(const ReloadPause &) = delete;
};
} // namespace tgskk::windows
