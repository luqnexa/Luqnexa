/* SPDX-License-Identifier: AGPL-3.0-only */
#ifndef NEXTIME_H
#define NEXTIME_H
#include <stddef.h>
#include <stdint.h>
#ifdef __cplusplus
extern "C" {
#endif
/* ABI v1. Integer handles must be destroyed once. Serialize all calls for one
   handle at the host level. Never race destroy with events or reads.
   Input spans must be readable; output spans/pointers writable and nonoverlapping.
   Paths/text are UTF-8 with explicit lengths, not NUL terminated inputs.
   Output strings have a trailing NUL, required size includes it. */
enum nextime_status {
    NEXTIME_OK=0, NEXTIME_INVALID_ARGUMENT=1, NEXTIME_INVALID_HANDLE=2,
    NEXTIME_INVALID_UTF8=3, NEXTIME_BUFFER_TOO_SMALL=4, NEXTIME_INPUT_LIMIT=5,
    NEXTIME_DICTIONARY_ERROR=6, NEXTIME_INTERNAL_ERROR=7
};
enum nextime_key {
    NEXTIME_TEXT=1, NEXTIME_BACKSPACE=2, NEXTIME_CONVERT=3, NEXTIME_NEXT=4,
    NEXTIME_PREVIOUS=5, NEXTIME_ENTER=6, NEXTIME_CANCEL=7,
    NEXTIME_LITERAL=8, NEXTIME_TOGGLE_MODE=9, NEXTIME_SPACE=10, NEXTIME_TOGGLE_AI=11,
    NEXTIME_CLEAR_CONTEXT=12, NEXTIME_CLEAR_LEARNING=13, NEXTIME_TOGGLE_TRAINING=14,
    NEXTIME_POLL_AI=15, /* Host timer only; never commits or sends a message. */
    NEXTIME_FULL_WIDTH=16, NEXTIME_HALF_WIDTH=17
};
enum nextime_text_kind {
    NEXTIME_RAW=1, NEXTIME_PREEDIT=2, NEXTIME_DISPLAY=3,
    NEXTIME_CANDIDATE=4, NEXTIME_COMMITTED=5, NEXTIME_MODE=6, NEXTIME_AI_MODE=7,
    NEXTIME_TRAINING_MODE=8, NEXTIME_LOCAL_AI_MODE=9
};
enum nextime_event_flags { NEXTIME_HANDLED=1, NEXTIME_HAS_COMMIT=2 };
uint32_t nextime_abi_version(void);
/* path_length=0 selects bundled dictionary; otherwise opens a NUD.
   limit=1..32. On failure out_handle=0. */
int32_t nextime_session_create(const uint8_t *path, size_t path_length,
                               uint32_t limit, uint64_t *out_handle);
int32_t nextime_session_destroy(uint64_t handle);
/* Only NEXTIME_TEXT accepts a nonempty text span; other events require length=0.
   Read NEXTIME_COMMITTED only if HAS_COMMIT is set; it lasts until next successful
   event. HANDLED=0 means host should pass the original key to the application. */
int32_t nextime_session_event(uint64_t handle, uint32_t key, const uint8_t *text,
                              size_t length, uint32_t *flags);
/* selected=UINT32_MAX if no candidate was explicitly selected (predictions may exist). */
int32_t nextime_session_candidates(uint64_t handle, uint32_t *count, uint32_t *selected);
/* Query required size with buffer=NULL/capacity=0 -> BUFFER_TOO_SMALL.
   Insufficient buffers are not modified. index applies only to CANDIDATE. */
int32_t nextime_session_text(uint64_t handle, uint32_t kind, uint32_t index,
                             uint8_t *buffer, size_t capacity, size_t *required);
#ifdef __cplusplus
}
#endif
#endif
