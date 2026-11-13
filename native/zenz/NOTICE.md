# Zenz runtime and model

The CPU worker uses [llama.cpp](https://github.com/ggml-org/llama.cpp) at revision `52e53a76df0298d2c316ae7e704eeec03b0bf478` (MIT).
The source is patched to recognize the `gpt2-small-japanese-char` pretokenizer as GPT-2, following the compatibility approach used in [Mozkey](https://github.com/koyasi777/mozkey).
The original llama.cpp license is installed alongside this notice.

The default model is **Zenz v3.2 small, Q5_K_M**, by **Keita Miwa**, from [Miwa-Keita/zenz-v3.2-small-gguf](https://huggingface.co/Miwa-Keita/zenz-v3.2-small-gguf), revision `c67e03e07d215c869f591b274c1631170d3e11fe`.
The model is licensed under [Creative Commons Attribution-ShareAlike 4.0 International](https://creativecommons.org/licenses/by-sa/4.0/legalcode.en).
The distributed quantized weights are downloaded unchanged.
SHA256: `29c223d4c23327b80fd13ebb5ab2555057a46317997d5da391584ffbef0db673`.
The model is separate from tgskk's GPL-3.0-or-later source code.

The prompt format, whitespace preprocessing, and candidate scoring without EOS follow [AzooKeyKanaKanjiConverter's Zenz implementation](https://github.com/azooKey/AzooKeyKanaKanjiConverter/tree/main/Sources/KanaKanjiConverterModule/ConversionAlgorithms/Zenzai/Zenz).
