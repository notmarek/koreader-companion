/*
 * Kindle's bundled libcjson.so is a custom old build that lacks the
 * typed convenience add functions (AddStringToObject, AddNumberToObject,
 * AddBoolToObject) added in cJSON 1.6.0, and uses different names for
 * type checks (cJSON_is_val_array instead of cJSON_IsArray).
 *
 * Provide implementations that are self-contained and don't depend on
 * any Amazon-specific naming, so the stubs work on all Kindle firmware
 * variants (kindlepw2 and kindlehf alike).
 *
 * cJSON type constants (stable across all known cJSON versions):
 *   cJSON_False=0, cJSON_True=1, cJSON_NULL=2, cJSON_Number=3,
 *   cJSON_String=4, cJSON_Array=5, cJSON_Object=6
 *
 * The cJSON struct layout (stable since cJSON's inception):
 *   struct cJSON { struct cJSON *next, *prev, *child; int type; ... }
 * The type field is at offset 12 on 32-bit ARM (3 pointers × 4 bytes).
 */

/* Forward declarations using only universally available cJSON primitives */
extern void *cJSON_CreateString(const char *str);
extern void *cJSON_CreateNumber(double num);
extern void *cJSON_CreateTrue(void);
extern void *cJSON_CreateFalse(void);
extern void *cJSON_CreateArray(void);
extern void  cJSON_AddItemToObject(void *object, const char *string, void *item);
extern void  cJSON_AddItemToArray(void *array, void *item);
extern void  free(void *ptr);

/* Read the type field from a cJSON node: next+prev+child = 3 pointers = 12 bytes on 32-bit */
static int cjson_type(const void *item)
{
    if (!item) return -1;
    return *((const int *)((const char *)item + 3 * sizeof(void *)));
}

void *cJSON_AddStringToObject(void *object, const char *name, const char *str)
{
    void *item = cJSON_CreateString(str);
    if (!item) return 0;
    cJSON_AddItemToObject(object, name, item);
    return item;
}

void *cJSON_AddNumberToObject(void *object, const char *name, double num)
{
    void *item = cJSON_CreateNumber(num);
    if (!item) return 0;
    cJSON_AddItemToObject(object, name, item);
    return item;
}

void *cJSON_AddBoolToObject(void *object, const char *name, int b)
{
    void *item = b ? cJSON_CreateTrue() : cJSON_CreateFalse();
    if (!item) return 0;
    cJSON_AddItemToObject(object, name, item);
    return item;
}

void *cJSON_CreateStringArray(const char *const *strings, int count)
{
    void *array = cJSON_CreateArray();
    int i;
    if (!array) return 0;
    for (i = 0; i < count; i++) {
        void *item = cJSON_CreateString(strings[i]);
        if (item) cJSON_AddItemToArray(array, item);
    }
    return array;
}

int cJSON_IsArray(const void *item)
{
    return cjson_type(item) == 5; /* cJSON_Array */
}

int cJSON_IsObject(const void *item)
{
    return cjson_type(item) == 6; /* cJSON_Object */
}

/* cJSON_Print returns malloc'd memory; free it the standard way */
void cJSON_free(void *ptr)
{
    free(ptr);
}
