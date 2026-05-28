#include <assert.h>
#include <math.h>
#include <stdatomic.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

typedef const void (*DropFn)(void*);
typedef const void (*CopyFn)(void*, void*);
typedef const bool (*EqualFn)(const void*, const void*);

/// A type-erased array
///
/// The storage of an array is a contiguous block of memory with the
/// following layout:
/// `{ ArrayHeader header; T payload[header.count]; }`
typedef struct Array {
    /// A pointer to the array's payload,
    /// i.e. the address of the array's storage offset by `sizeof(ArrayHeader)`
    ///
    /// If the pointer is null, then it is assumed that the array has a 0 count and capacity
    void* payload;
} Array;

/// The header of an array
typedef struct ArrayHeader {
    /// The number of references to the array's storage.
    _Atomic(uint64_t) refc;
    /// The number of elements currently in the array
    uint64_t count;
    /// The total capacity of the array, in bytes. Should always be >= count * sizeof(T)
    uint64_t capacity;
} ArrayHeader;

static inline ArrayHeader* get_array_header(Array* array) {
    if (array->payload == NULL) return NULL;
    return (ArrayHeader*)((uint8_t*)array->payload - sizeof(ArrayHeader));
}

static inline uint64_t get_capacity(uint64_t count, uint64_t elem_size) {
    if (count == 1) {
        // Special-case optimisation for 1-element arrays
        if (elem_size == 1) {
            return 8; // Round up to 8 bytes because the allocator probably will anyway
        } else if (elem_size <= 1024) {
            return 4 * elem_size; // 4 is a nice balance for medium-size elements
        } else {
            return elem_size; // For >1kb elements, just use 1 to avoid wasting too much memory
        }
    } else if (count < 8) {
        return 8 * elem_size;
    } else {
        // If the count isn't a power of 2, round it up to one
        // (https://stackoverflow.com/a/466242)
        if ((count & (count - 1)) != 0) {
            count--;
            count |= count >> 1;
            count |= count >> 2;
            count |= count >> 4;
            count |= count >> 8;
            count |= count >> 16;
            count |= count >> 32;
            count++;
        }

        return count * elem_size;
    }
}

void _array_drop(Array* array, DropFn elem_drop, uint64_t elem_size) {
    // Don't do anything if the array storage is unallocated (empty array)
    ArrayHeader* header = get_array_header(array);
    if (header == NULL) return;

    // Decrement the ref count
    int old_val = atomic_fetch_sub_explicit(&header->refc, 1, memory_order_acq_rel);

    // If the ref count reached zero, we drop each element if needed and
    // deallocate the storage
    if (old_val == 1) {
        if (elem_drop != NULL) {
            uint8_t* payload = (uint8_t*)array->payload;
            for (size_t i = 0; i < header->count; ++i) {
                elem_drop(&payload[i * elem_size]);
            }
        }

        free(header);
        array->payload = NULL;
    }
}

void _array_copy(Array* dst, Array* src) {
    // Copy the stack allocation of the array
    *dst = *src;

    // Increment the ref count
    ArrayHeader* header = get_array_header(src);
    if (header == NULL) return;
    uint64_t old_val = atomic_fetch_add_explicit(&header->refc, 1, memory_order_relaxed);
}

bool _array_equals(Array* lhs, Array* rhs, EqualFn elem_equals, uint64_t elem_size) {
    uint8_t* lhs_payload = (uint8_t*)lhs->payload;
    uint8_t* rhs_payload = (uint8_t*)rhs->payload;
    // True if the arrays share storage. This will also catch if both payloads are null
    if (lhs_payload == rhs_payload) return true;

    ArrayHeader* lhs_header = get_array_header(lhs);
    ArrayHeader* rhs_header = get_array_header(rhs);
    // If they have different numbers of elements, always false
    if (lhs_header->count != rhs_header->count) return false;

    // Check equality for each element individually
    for (uint64_t i = 0; i < lhs_header->count; ++i) {
        void* lhs_elem = &lhs_payload[i * elem_size];
        void* rhs_elem = &rhs_payload[i * elem_size];
        if (!elem_equals(lhs_elem, rhs_elem)) {
            return false;
        }
    }
    return true;
}

void _array_unique(Array* array, CopyFn elem_copy, uint64_t elem_size) {
    // If the array is empty or it's already unique, don't need to do anything
    ArrayHeader* old_header = get_array_header(array);
    if (old_header == NULL) return;
    if (atomic_load_explicit(&old_header->refc, memory_order_acquire) == 1) return;

    // Allocate new storage with room for the header plus the capacity of the array being copied
    void* new_storage = malloc(sizeof(ArrayHeader) + old_header->capacity);

    // Initialize the new header
    ArrayHeader* new_header = new_storage;
    new_header->refc = 1;
    new_header->count = old_header->count;
    new_header->capacity = old_header->capacity;

    // Initialise the new payload with copies of the current elements
    uint8_t* new_payload = new_storage + sizeof(ArrayHeader);
    if (elem_copy == NULL) {
        memcpy(new_payload, array->payload, old_header->capacity);
    } else {
        uint8_t* src = array->payload;
        for (size_t i = 0; i < old_header->count; ++i) {
            elem_copy(&new_payload[i * elem_size], &src[i * elem_size]);
        }
    }

    // Insert the new storage and decrement the ref count on the old storage
    array->payload = new_payload;
    atomic_fetch_sub_explicit(&old_header->refc, 1, memory_order_acq_rel);
}

void _array_new(Array* array, uint64_t count, uint64_t elem_size) {
    if (count > 0) {
        // Allocate the storage
        uint64_t capacity = get_capacity(count, elem_size);
        void* storage = malloc(sizeof(ArrayHeader) + capacity);
        array->payload = storage + sizeof(ArrayHeader);

        // Initialize the header
        ArrayHeader* header = storage;
        header->refc = 1;
        header->count = count;
        header->capacity = capacity;

        // Initialize the payload
        uint8_t* payload = (uint8_t*)array->payload;
        memset(payload, 0, capacity);
    } else {
        array->payload = NULL;
    }
}