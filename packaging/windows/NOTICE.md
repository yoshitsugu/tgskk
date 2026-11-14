# Windows dictionary

The package includes SKK-JISYO.L from [skk-dev/dict](https://github.com/skk-dev/dict), revision `14a1df7ec8f84410f5fb006978cf00878768150a`, unchanged.
SHA256: `c791f578d1b4040fce282db29bc22b2cc7ea46f83e269fab2e0fa779e2967e40`.
The dictionary's original copyright and GPL-2.0-or-later notice are retained in its header.
This distribution uses the GPL-3.0-or-later license supplied as `LICENSE`.
The Unicode data notice is supplied separately as `dictionary-Unicode-LICENSE.txt`.

# Runtime and source

Unmodified Microsoft Visual C++ runtime DLLs are redistributed app-locally from the Visual Studio redistributable CRT directory.
They retain Microsoft's embedded copyright notices and are separate from tgskk's GPL license.
Rust dependency notices and SPDX metadata are provided under `licenses/rust`.
The companion `tgskk-source.tar.gz` includes the exact project revision and all locked Rust registry sources under `vendor`; its Cargo configuration supports offline dependency resolution.
The pinned llama.cpp source archive is supplied alongside it; the tokenizer patch and build commands are in `scripts/setup-zenz.py`.
