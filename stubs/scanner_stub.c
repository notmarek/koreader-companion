/* Stub for libscanner.so — resolved at runtime on the Kindle */
int   scanner_post_change(void* json) { return 0; }
void  scanner_gen_uuid(char* out, int size) {}
char* scanner_get_thumbnail_for_uuid(char* uuid) { return 0; }
void  scanner_update_ccat_entry_with_thumbpath(char* uuid, char* path) {}
void  scanner_delete_ccat_entry(char* uuid) {}
char* getSha1Hash(const char* data) { return 0; }
