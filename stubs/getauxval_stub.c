/* kindlepw2 sysroot uses glibc 2.12; getauxval was added in glibc 2.16.
 * Confirmed missing on firmware 5.16.2.1.1 (Kindle Oasis 9th gen / kindlepw2).
 * Return 0 for all queries — safe since we don't use it for capability detection. */
unsigned long getauxval(unsigned long type) { return 0; }
