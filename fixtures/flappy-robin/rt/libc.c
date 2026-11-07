// The few C runtime functions the game needs, so it builds without newlib.
#include <stddef.h>
#include <stdint.h>

// VRAM and palette RAM ignore byte writes, so copy in the widest unit the
// alignment allows.
void *memcpy(void *dst, const void *src, size_t n) {
	uintptr_t a = (uintptr_t)dst | (uintptr_t)src | n;
	if ((a & 3) == 0) {
		uint32_t *d = dst; const uint32_t *s = src;
		for (; n; n -= 4) *d++ = *s++;
	} else if ((a & 1) == 0) {
		uint16_t *d = dst; const uint16_t *s = src;
		for (; n; n -= 2) *d++ = *s++;
	} else {
		uint8_t *d = dst; const uint8_t *s = src;
		while (n--) *d++ = *s++;
	}
	return dst;
}

void *memset(void *dst, int c, size_t n) {
	uint8_t *d = dst;
	while (n--) *d++ = (uint8_t)c;
	return dst;
}

void __aeabi_memcpy(void *d, const void *s, size_t n) { memcpy(d, s, n); }
void __aeabi_memcpy4(void *d, const void *s, size_t n) { memcpy(d, s, n); }
void __aeabi_memcpy8(void *d, const void *s, size_t n) { memcpy(d, s, n); }
void __aeabi_memset(void *d, size_t n, int c) { memset(d, c, n); }
void __aeabi_memset4(void *d, size_t n, int c) { memset(d, c, n); }
void __aeabi_memset8(void *d, size_t n, int c) { memset(d, c, n); }
void __aeabi_memclr(void *d, size_t n) { memset(d, 0, n); }
void __aeabi_memclr4(void *d, size_t n) { memset(d, 0, n); }
void __aeabi_memclr8(void *d, size_t n) { memset(d, 0, n); }

static uint32_t uquot(uint32_t n, uint32_t d, uint32_t *rem) {
	uint32_t q = 0, r = 0;
	for (int i = 31; i >= 0; i--) {
		r = (r << 1) | ((n >> i) & 1);
		if (r >= d) { r -= d; q |= 1u << i; }
	}
	*rem = r;
	return q;
}

uint32_t __aeabi_uidiv(uint32_t n, uint32_t d) {
	uint32_t r;
	return uquot(n, d, &r);
}

int32_t __aeabi_idiv(int32_t n, int32_t d) {
	uint32_t r, q = uquot(n < 0 ? -(uint32_t)n : (uint32_t)n, d < 0 ? -(uint32_t)d : (uint32_t)d, &r);
	return ((n < 0) != (d < 0)) ? -(int32_t)q : (int32_t)q;
}

// AEABI: the quotient comes back in r0 and the remainder in r1.
uint64_t __aeabi_uidivmod(uint32_t n, uint32_t d) {
	uint32_t r, q = uquot(n, d, &r);
	return ((uint64_t)r << 32) | q;
}

uint64_t __aeabi_idivmod(int32_t n, int32_t d) {
	uint32_t r, q = uquot(n < 0 ? -(uint32_t)n : (uint32_t)n, d < 0 ? -(uint32_t)d : (uint32_t)d, &r);
	int32_t sq = ((n < 0) != (d < 0)) ? -(int32_t)q : (int32_t)q;
	int32_t sr = n < 0 ? -(int32_t)r : (int32_t)r;
	return ((uint64_t)(uint32_t)sr << 32) | (uint32_t)sq;
}

static uint32_t rand_state = 1;
void srand(unsigned seed) { rand_state = seed; }
int rand(void) {
	rand_state = rand_state * 1103515245u + 12345u;
	return (rand_state >> 16) & 0x7FFF;
}
