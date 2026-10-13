/* SPDX-License-Identifier: AGPL-3.0-only */
#ifndef LUQNEXA_MODEL_H
#define LUQNEXA_MODEL_H
#include "nextime.h"
#ifdef __cplusplus
extern "C" {
#endif
/* Only available when built with --features mobile-neural.
   Explicit loading: no filesystem, network or personal-history access.
   Verify provenance/signatures at the host before passing model bytes.
   Serialize calls for a handle; destroy after in-flight calls finish.
   At most four loaded models. Failed create sets out_handle=0.
   Input pointers must designate readable spans and outputs writable memory. */
#define LUQNEXA_MODEL_BYTES 165692u
int32_t luqnexa_model_create(const uint8_t *bytes, size_t length, uint64_t *out_handle);
int32_t luqnexa_model_destroy(uint64_t handle);
/* UTF-8, explicit byte lengths; max reading/context/surface: 256/4096/384 bytes.
   Model uses first 64 reading, last 48 context, first 96 surface Unicode scalars.
   Higher score is preferred; this supplies only learned scores, not conversion.
   The host preserves the existing candidates and user confirms insertion.
   Failed score leaves out_score unchanged. Empty spans may have NULL pointers. */
int32_t luqnexa_model_score(uint64_t handle,
    const uint8_t *reading, size_t reading_length,
    const uint8_t *context, size_t context_length,
    const uint8_t *surface, size_t surface_length, float *out_score);
#ifdef __cplusplus
}
#endif
#endif
