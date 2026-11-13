#ifndef TGSKK_PATHS_H
#define TGSKK_PATHS_H

#include <filesystem>
#if __has_include(<fcitx-utils/standardpaths.h>)
#include <fcitx-utils/standardpaths.h>
#else
#include <fcitx-utils/standardpath.h>
#endif

namespace tgskk {
#if __has_include(<fcitx-utils/standardpaths.h>)
using Paths = fcitx::StandardPaths;
using PathType = fcitx::StandardPathsType;
#else
using Paths = fcitx::StandardPath;
using PathType = fcitx::StandardPath::Type;
#endif

// Normalize the string paths in fcitx 5.1.7 to the newer filesystem API.
inline std::filesystem::path userDataDirectory() {
  return Paths::global().userDirectory(PathType::PkgData);
}
inline std::filesystem::path systemPath(const char *name) {
  return Paths::fcitxPath(name);
}
} // namespace tgskk
#endif
