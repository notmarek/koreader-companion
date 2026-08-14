/* Stub for liblipc.so — resolved at runtime on the Kindle */
void* LipcOpenEx(const char* s, int* c) { return 0; }
void  LipcClose(void* l) {}
int   LipcRegisterStringProperty(void* l, const char* p, void* g, void* s, void* d) { return 0; }
int   LipcSetStringProperty(void* l, const char* svc, const char* p, const char* v) { return 0; }
int   LipcSetIntProperty(void* l, const char* svc, const char* p, int v) { return 0; }
int   LipcGetStringProperty(void* l, const char* svc, const char* p, char** v) { return 0; }
void  LipcFreeString(char* s) {}
