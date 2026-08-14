/*
 * Kindle's bundled libcjson.so is a custom old build that lacks the
 * convenience add functions and uses different type-check naming.
 * Provide implementations using the primitives that DO exist so our
 * extractor SO is self-contained and doesn't fail dlopen at runtime.
 *
 * Available on Kindle: cJSON_CreateObject, cJSON_CreateArray,
 *   cJSON_CreateString, cJSON_CreateNumber, cJSON_CreateTrue,
 *   cJSON_CreateFalse, cJSON_AddItemToObject, cJSON_AddItemToArray,
 *   cJSON_Print, cJSON_Delete, cJSON_is_val_array, cJSON_is_val_object
 */

/* Forward declarations of functions that DO exist on the Kindle */
extern void *cJSON_CreateString(const char *str);
extern void *cJSON_CreateNumber(double num);
extern void *cJSON_CreateTrue(void);
extern void *cJSON_CreateFalse(void);
extern void *cJSON_CreateArray(void);
extern void  cJSON_AddItemToObject(void *object, const char *string, void *item);
extern void  cJSON_AddItemToArray(void *array, void *item);
extern int   cJSON_is_val_array(const void *item);
extern int   cJSON_is_val_object(const void *item);

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
    return cJSON_is_val_array(item);
}

int cJSON_IsObject(const void *item)
{
    return cJSON_is_val_object(item);
}

/* cJSON_Print returns malloc'd memory; free it the standard way */
extern void free(void *ptr);
void cJSON_free(void *ptr)
{
    free(ptr);
}
