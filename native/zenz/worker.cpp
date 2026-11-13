// SPDX-License-Identifier: GPL-3.0-or-later
#include "llama.h"
#include "ggml-cpu.h"
#include <algorithm>
#include <cmath>
#include <cstdint>
#include <cstdio>
#include <cstring>
#include <iostream>
#include <memory>
#include <stdexcept>
#include <string>
#include <vector>
#include <thread>

namespace {
constexpr uint32_t maxString = 1024;
constexpr uint32_t maxCandidates = 8;
// Increment the protocol version whenever request/response framing changes.
constexpr uint32_t protocolMagic = 0x325A4754; // TGZ2, little-endian.
constexpr uint32_t rejectedRequest = UINT32_MAX;
class InvalidInput : public std::runtime_error {
public:
    using std::runtime_error::runtime_error;
};

bool readWord(uint32_t &value) {
    unsigned char bytes[4];
    if (std::fread(bytes, 1, 4, stdin) != 4) return false;
    value = uint32_t(bytes[0]) | uint32_t(bytes[1]) << 8 |
            uint32_t(bytes[2]) << 16 | uint32_t(bytes[3]) << 24;
    return true;
}
void writeWord(uint32_t value) {
    unsigned char bytes[4];
    for (int i = 0; i < 4; ++i) bytes[i] = value >> (i * 8);
    if (std::fwrite(bytes, 1, 4, stdout) != 4) throw std::runtime_error("write");
}
std::string readString() {
    uint32_t size;
    if (!readWord(size) || size > maxString) throw std::runtime_error("string size");
    std::string text(size, '\0');
    if (std::fread(text.data(), 1, size, stdin) != size) throw std::runtime_error("string");
    return text;
}
void writeScore(double score) {
    static_assert(sizeof(double) == 8);
    uint64_t bits;
    std::memcpy(&bits, &score, 8);
    writeWord(uint32_t(bits));
    writeWord(uint32_t(bits >> 32));
}
std::vector<llama_token> tokenize(const llama_vocab *vocab, const std::string &text, bool bos) {
    std::vector<llama_token> tokens(text.size() + 32);
    int count = llama_tokenize(vocab, text.data(), text.size(), tokens.data(), tokens.size(), bos, false);
    if (count < 0) throw InvalidInput("tokenize");
    tokens.resize(count);
    return tokens;
}
std::string preprocess(const std::string &text) {
    std::string out;
    for (size_t i = 0; i < text.size(); ++i) {
        if (i + 2 < text.size() && static_cast<unsigned char>(text[i]) == 0xee &&
            static_cast<unsigned char>(text[i + 1]) == 0xb8 &&
            static_cast<unsigned char>(text[i + 2]) >= 0x80 &&
            static_cast<unsigned char>(text[i + 2]) <= 0x8f) { i += 2; continue; }
        const char ch = text[i];
        if (ch == ' ') out += "　";
        else if (ch != '\n' && ch != '\r') out += ch;
    }
    return out;
}
double logProbability(const float *logits, int size, int token) {
    float maximum = *std::max_element(logits, logits + size);
    double sum = 0;
    for (int i = 0; i < size; ++i) sum += std::exp(double(logits[i] - maximum));
    return logits[token] - maximum - std::log(sum);
}
class Scorer {
    std::unique_ptr<llama_model, decltype(&llama_model_free)> model_{nullptr, llama_model_free};
    std::unique_ptr<ggml_threadpool, decltype(&ggml_threadpool_free)> pool_{nullptr, ggml_threadpool_free};
    std::unique_ptr<llama_context, decltype(&llama_free)> context_{nullptr, llama_free};
    const llama_vocab *vocab_ = nullptr;
    std::vector<llama_token> prefix_;
    std::vector<float> next_;

    void decode(const std::vector<llama_token> &tokens, size_t position, bool all) {
        auto batch = llama_batch_init(tokens.size(), 0, 1);
        batch.n_tokens = tokens.size();
        for (int i = 0; i < batch.n_tokens; ++i) {
            batch.token[i] = tokens[i];
            batch.pos[i] = position + i;
            batch.n_seq_id[i] = 1;
            batch.seq_id[i][0] = 0;
            batch.logits[i] = all || i == batch.n_tokens - 1;
        }
        const int result = llama_decode(context_.get(), batch);
        llama_batch_free(batch);
        if (result) throw std::runtime_error("decode");
    }
    void prepare(const std::vector<llama_token> &tokens) {
        if (tokens == prefix_) return;
        size_t common = 0;
        while (common < prefix_.size() && common < tokens.size() && prefix_[common] == tokens[common]) ++common;
        // Re-evaluate the last token when shortening so its logits are available.
        if (common == tokens.size()) --common;
        if (!llama_memory_seq_rm(llama_get_memory(context_.get()), 0, common, -1))
            throw std::runtime_error("trim prefix");
        decode({tokens.begin() + common, tokens.end()}, common, false);
        const auto *next = llama_get_logits_ith(context_.get(), -1);
        next_.assign(next, next + llama_vocab_n_tokens(vocab_));
        prefix_ = tokens;
    }
public:
    Scorer(const char *path, int threads) {
        auto mp = llama_model_default_params();
        mp.n_gpu_layers = 0;
        model_.reset(llama_model_load_from_file(path, mp));
        if (!model_) throw std::runtime_error("model");
        auto cp = llama_context_default_params();
        cp.n_ctx = 1024;
        cp.n_batch = 512;
        cp.n_ubatch = 512;
        cp.n_threads = threads;
        cp.n_threads_batch = threads;
        cp.no_perf = true;
        context_.reset(llama_init_from_model(model_.get(), cp));
        if (!context_) throw std::runtime_error("context");
        auto params = ggml_threadpool_params_default(threads);
        params.poll = 0;
        pool_.reset(ggml_threadpool_new(&params));
        if (!pool_) throw std::runtime_error("thread pool");
        llama_attach_threadpool(context_.get(), pool_.get(), pool_.get());
        vocab_ = llama_model_get_vocab(model_.get());
    }
    std::vector<double> score(const std::string &left, const std::string &reading,
                              const std::vector<std::string> &candidates) {
        std::string prompt;
        if (!left.empty()) prompt = "\xee\xb8\x82" + preprocess(left);
        prompt += "\xee\xb8\x80" + preprocess(reading);
        if (!candidates.empty()) prompt += "\xee\xb8\x81";
        const auto tokens = tokenize(vocab_, prompt, true);
        if (tokens.empty() || tokens.size() > 384) throw InvalidInput("prompt size");
        prepare(tokens);
        std::vector<double> scores;
        for (const auto &candidate : candidates) {
            // Zenz evaluates candidate tokens without EOS; ending probability
            // would incorrectly penalize words that normally continue a sentence.
            const auto tail = tokenize(vocab_, preprocess(candidate), false);
            if (tail.empty() || tail.size() > 96) throw InvalidInput("candidate size");
            double result = logProbability(next_.data(), next_.size(), tail[0]);
            if (tail.size() > 1) {
                decode({tail.begin(), tail.end() - 1}, prefix_.size(), true);
                for (size_t i = 1; i < tail.size(); ++i)
                    result += logProbability(llama_get_logits_ith(context_.get(), i - 1), next_.size(), tail[i]);
                if (!llama_memory_seq_rm(llama_get_memory(context_.get()), 0, prefix_.size(), -1))
                    throw std::runtime_error("trim candidate");
            }
            scores.push_back(result);
        }
        return scores;
    }
};
}
int main(int argc, char **argv) {
    if (argc != 2) { std::cerr << "Usage: tgskk-zenz-worker MODEL.gguf\n"; return 2; }
    llama_log_set([](ggml_log_level level, const char *, void *) {
        if (level == GGML_LOG_LEVEL_ERROR) std::fputs("Zenz runtime error\n", stderr);
    }, nullptr);
    try {
        llama_backend_init();
        const auto available = std::thread::hardware_concurrency();
        const int threads = available == 0 ? 2 : std::max(1u, std::min(4u, available / 2));
        Scorer scorer(argv[1], threads);
        writeWord(protocolMagic);
        std::fflush(stdout);
        uint32_t count;
        while (readWord(count)) {
            if (count > maxCandidates) throw std::runtime_error("candidate count");
            const auto left = readString();
            const auto reading = readString();
            std::vector<std::string> candidates;
            for (uint32_t i = 0; i < count; ++i) candidates.push_back(readString());
            try {
                const auto scores = scorer.score(left, reading, candidates);
                writeWord(scores.size());
                for (double score : scores) writeScore(score);
            } catch (const InvalidInput &) {
                writeWord(rejectedRequest);
            }
            if (std::fflush(stdout)) return 1;
        }
        return 0;
    } catch (const std::exception &) {
        std::fputs("Zenz worker failed; using dictionary order\n", stderr);
        return 1;
    }
}
