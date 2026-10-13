/* SPDX-License-Identifier: AGPL-3.0-only */
#include "luqnexa_model.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <math.h>
int main(int argc, char **argv) {
    if (argc != 2) { fprintf(stderr, "usage: neural-demo model.lqml\n"); return 1; }
    FILE *file = fopen(argv[1], "rb");
    if (!file) return 2;
    uint8_t *bytes = (uint8_t *)malloc(LUQNEXA_MODEL_BYTES);
    if (!bytes) { fclose(file); return 3; }
    size_t size = fread(bytes, 1, LUQNEXA_MODEL_BYTES, file);
    int trailing = fgetc(file);
    fclose(file);
    uint64_t handle = 0;
    int32_t status = luqnexa_model_create(bytes, size, &handle);
    free(bytes);
    if (trailing != EOF || status != NEXTIME_OK || handle == 0) return 4;
    const char *reading = "hashi";
    const char *context = "夕食";
    const char *surface = "箸";
    float score = 0;
    status = luqnexa_model_score(handle,
        (const uint8_t *)reading, strlen(reading),
        (const uint8_t *)context, strlen(context),
        (const uint8_t *)surface, strlen(surface), &score);
    if (status != NEXTIME_OK || !isfinite(score)) return 5;
    printf("Rust neural C ABI: %s score=%f model_bytes=%u\n", surface, (double)score, LUQNEXA_MODEL_BYTES);
    if (luqnexa_model_destroy(handle) != NEXTIME_OK) return 6;
    if (luqnexa_model_destroy(handle) != NEXTIME_INVALID_HANDLE) return 7;
    return 0;
}
