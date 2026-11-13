#include <stdio.h>
#include <stdlib.h>

__attribute__((constructor)) static void loaded(void) {
    const char *path = getenv("TGSKK_PROBE_MARKER");
    if (path) {
        FILE *file = fopen(path, "w");
        if (file) { fputs("unexpected dynamic backend\n", file); fclose(file); }
    }
}
