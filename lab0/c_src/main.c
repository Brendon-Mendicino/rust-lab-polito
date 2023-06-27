#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include <time.h>
#include <string.h>


typedef enum export_type_e {
    VALUE = 1,
    M_VALUE,
    MESSAGE
} export_type_e;

#define EXPORT_STRINGS_LEN (7)

char export_strings[EXPORT_STRINGS_LEN][21] = {
    "Bella",
    "Test",
    "Pippo",
    "Pluto",
    "42",
    "AnswerToTheUniverse",
    "E tutto il resto...",
};

#define FLOAT_ARRAY_LEN (10)


float float_array[FLOAT_ARRAY_LEN] = {
    1.0,
    2.0,
    3.0,
    4.0,
    5.0,
    6.0,
    7.0,
    8.0,
    9.0,
    10.0,
};


typedef struct {
    int type;
    float val;
    long timestamp;
} ValueStruct;

typedef struct {
    int type;
    float val[10];
    long timestamp;
} MValueStruct;

typedef struct {
    int type;
    char message[21]; // stringa null terminated lung max 20
} MessageStruct;

typedef struct {
    int type;
    union {
        ValueStruct val;
        MValueStruct mvals;
        MessageStruct messages;
    };
} ExportData;


ValueStruct get_value_struct(void) {
    static int val_counter = 0;

    ValueStruct value = (ValueStruct){
        .type = VALUE,
        .val = float_array[val_counter],
        .timestamp = time(NULL),
    };

    val_counter = (val_counter + 1) % FLOAT_ARRAY_LEN;

    return value;
}

MValueStruct get_m_value_struct(void) {

    MValueStruct value = (MValueStruct) {
        .type = M_VALUE,
        .timestamp = time(NULL),
    };
    memcpy(value.val, float_array, sizeof(value.val));

    return value;
}

MessageStruct get_message_struct(void) {
    static int string_counter = 0;

    MessageStruct message = (MessageStruct) {
        .type = MESSAGE,
    };
    memcpy(message.message, export_strings[string_counter], sizeof(message.message));

    string_counter = (string_counter + 1) % EXPORT_STRINGS_LEN;

    return message;
}

void fill_export_data(ExportData *data, uint32_t len) {
    for (int i = 0; i < len; i++) {
        ExportData export;

        switch (i % 3) {
            case 0: {
                export = (ExportData){
                    .type = VALUE,
                    .val = get_value_struct(),
                };
                break;
            }
            case 1: {
                export = (ExportData){
                    .type = M_VALUE,
                    .mvals = get_m_value_struct(),
                };
                break;
            }
            case 2: {
                export = (ExportData){
                    .type = MESSAGE,
                    .messages = get_message_struct(),
                };
                break;
            }
        }

        printf("%d %d\n", i, export.type);

        data[i] = export;
    }
}

void export(ExportData *data, uint32_t data_len, FILE *fp) {
    fwrite(data, sizeof(ExportData), data_len, fp);
}

#define EXPORT_DATA_LEN (100)

int main(int argc, char **argv) {
    
    ExportData *data = (ExportData *)malloc(sizeof(ExportData) * EXPORT_DATA_LEN);

    fill_export_data(data, EXPORT_DATA_LEN);

    FILE *fp = fopen("../data", "w");

    export(data, EXPORT_DATA_LEN, fp);

    free(data);

    return 0;
}
