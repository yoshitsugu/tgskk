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
#ifdef _WIN32
#include <fcntl.h>
#include <io.h>
#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#ifndef NOMINMAX
#define NOMINMAX
#endif
#include <windows.h>
#endif

namespace {
constexpr uint32_t maxString = 1024;
constexpr uint32_t maxCandidates = 8;
// Measured with GGML_NATIVE=OFF/AVX2: combining 8+ tokens changes scores.
// Keep each candidate intact to preserve serial scoring for longer candidates.
#if defined(__x86_64__) || defined(_M_X64)
constexpr size_t maxCandidateBatchTokens = 7;
#else
constexpr size_t maxCandidateBatchTokens = 1;
#endif
// Longer prompts change scores through the unified KV path. Normal SKK input fits below this bound.
constexpr size_t maxPromptTokens = 252;
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
        cp.n_seq_max = maxCandidates + 1;
        cp.kv_unified = true;
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
        std::vector<std::vector<llama_token>> tails;
        for (const auto &candidate : candidates) {
            tails.push_back(tokenize(vocab_, preprocess(candidate), false));
            if (tails.back().empty() || tails.back().size() > 96)
                throw InvalidInput("candidate size");
        }
        std::string prompt;
        if (!left.empty()) prompt = "\xee\xb8\x82" + preprocess(left);
        prompt += "\xee\xb8\x80" + preprocess(reading);
        if (!candidates.empty()) prompt += "\xee\xb8\x81";
        const auto tokens = tokenize(vocab_, prompt, true);
        if (tokens.empty() || tokens.size() > maxPromptTokens) throw InvalidInput("prompt size");
        prepare(tokens);
        std::vector<double> scores;
        // EOS would penalize words that normally continue a sentence.
        for (const auto &tail : tails)
            scores.push_back(logProbability(next_.data(), next_.size(), tail[0]));
        auto *memory = llama_get_memory(context_.get());
        for (size_t begin = 0; begin < tails.size();) {
            size_t end = begin, rows = 0;
            while (end < tails.size()) {
                const size_t need = tails[end].size() - 1;
                if (rows > 0 && rows + need > maxCandidateBatchTokens) break;
                rows += need;
                ++end;
            }
            if (rows > 0) {
                auto batch = llama_batch_init(rows, 0, 1);
                std::vector<size_t> first(tails.size());
                for (size_t k = begin; k < end; ++k) {
                    first[k] = batch.n_tokens;
                    if (tails[k].size() < 2) continue;
                    llama_memory_seq_cp(memory, 0, k + 1, -1, -1);
                    for (size_t i = 0; i + 1 < tails[k].size(); ++i) {
                        const int n = batch.n_tokens++;
                        batch.token[n] = tails[k][i];
                        batch.pos[n] = prefix_.size() + i;
                        batch.n_seq_id[n] = 1;
                        batch.seq_id[n][0] = k + 1;
                        batch.logits[n] = true;
                    }
                }
                const int result = llama_decode(context_.get(), batch);
                llama_batch_free(batch);
                if (result) throw std::runtime_error("decode");
                for (size_t k = begin; k < end; ++k) {
                    for (size_t i = 1; i < tails[k].size(); ++i)
                        scores[k] += logProbability(llama_get_logits_ith(context_.get(), first[k] + i - 1), next_.size(), tails[k][i]);
                    if (tails[k].size() > 1 && !llama_memory_seq_rm(memory, k + 1, -1, -1))
                        throw std::runtime_error("trim candidate");
                }
            }
            begin = end;
        }
        return scores;
    }
};
}
int workerMain(int argc, char **argv) {
#ifdef _WIN32
    if (_setmode(_fileno(stdin), _O_BINARY) == -1 || _setmode(_fileno(stdout), _O_BINARY) == -1) return 1;
#endif
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

#ifdef _WIN32
int wmain(int argc, wchar_t **argv) {
    std::vector<std::string> values;
    for (int i = 0; i < argc; ++i) {
        int n = WideCharToMultiByte(CP_UTF8, WC_ERR_INVALID_CHARS, argv[i], -1, nullptr, 0, nullptr, nullptr);
        if (!n) return 1;
        std::string value(n, '\0');
        if (!WideCharToMultiByte(CP_UTF8, WC_ERR_INVALID_CHARS, argv[i], -1, value.data(), n, nullptr, nullptr)) return 1;
        value.pop_back(); values.push_back(std::move(value));
    }
    std::vector<char *> pointers;
    for (auto &value : values) pointers.push_back(value.data());
    pointers.push_back(nullptr);
    return workerMain(argc, pointers.data());
}
#else
int main(int argc, char **argv) { return workerMain(argc, argv); }
#endif
