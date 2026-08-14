/*
 * Regression test for stubs/cjson_addboolfixup.c.
 *
 * The stub's cJSON_IsObject / cJSON_IsArray must accept BOTH type-constant
 * conventions:
 *   - modern cJSON (1.7.x): bitflag constants, Object=64, Array=32
 *   - Amazon's ancient fork (pre-1.6): sequential constants, Object=6, Array=5
 *
 * Items are created with the modern system libcjson (linked via -lcjson);
 * the stub's checks are linked in statically, so calls to cJSON_IsObject /
 * cJSON_IsArray resolve to the stub — exactly the configuration that broke
 * on devices shipping modern libcjson.
 *
 * Build/run (host):
 *   gcc tests/cjson_stub_test.c stubs/cjson_addboolfixup.c \
 *       $(pkg-config --cflags --libs libcjson) -o /tmp/cjson_stub_test \
 *   && /tmp/cjson_stub_test
 */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <cjson/cJSON.h>

static int failures = 0;

static void check(int cond, const char *what)
{
    if (cond) {
        printf("ok   - %s\n", what);
    } else {
        printf("FAIL - %s\n", what);
        failures++;
    }
}

/* Ancient Amazon cJSON fork layout: type field at offset 3*sizeof(void*),
 * sequential constants (Array=5, Object=6). Same layout prefix as modern
 * cJSON, so the stub's offset read works for both. */
struct fake_old_item {
    void *next, *prev, *child;
    int type;
};

int main(void)
{
    /* modern libcjson items (bitflag constants) */
    cJSON *obj = cJSON_CreateObject();
    cJSON *arr = cJSON_CreateArray();
    cJSON *str_item = cJSON_CreateString("hello");
    check(obj != NULL && arr != NULL && str_item != NULL, "modern cJSON creates items");

    check(cJSON_IsObject(obj) == 1, "stub IsObject accepts modern object (type 64)");
    check(cJSON_IsArray(arr) == 1, "stub IsArray accepts modern array (type 32)");

    /* simulated ancient-fork items (sequential constants) */
    struct fake_old_item old_obj = {0, 0, 0, 6};
    struct fake_old_item old_arr = {0, 0, 0, 5};
    check(cJSON_IsObject((const cJSON *)&old_obj) == 1,
          "stub IsObject accepts old-fork object (type 6)");
    check(cJSON_IsArray((const cJSON *)&old_arr) == 1,
          "stub IsArray accepts old-fork array (type 5)");

    /* negatives: the checks must not be trivially always-true */
    check(cJSON_IsObject(NULL) == 0, "stub IsObject rejects NULL");
    check(cJSON_IsArray(NULL) == 0, "stub IsArray rejects NULL");
    check(cJSON_IsObject(str_item) == 0, "stub IsObject rejects string item");
    check(cJSON_IsArray(str_item) == 0, "stub IsArray rejects string item");

    /* stub's convenience add path against modern lib */
    check(cJSON_AddStringToObject(obj, "type", "ChangeRequest") != NULL,
          "stub AddStringToObject works against modern lib");
    check(cJSON_IsObject(obj) == 1, "object still IsObject after add");

    char *printed = cJSON_PrintUnformatted(obj);
    check(printed != NULL, "modern lib prints object");
    if (printed) {
        check(strstr(printed, "\"type\"") != NULL, "printed JSON contains added key");
        free(printed);
    }

    cJSON_Delete(obj);
    cJSON_Delete(arr);
    cJSON_Delete(str_item);

    if (failures) {
        printf("%d check(s) FAILED\n", failures);
        return 1;
    }
    printf("all cjson stub checks passed\n");
    return 0;
}
