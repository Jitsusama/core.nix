// What the computer can ask of the board over Oryx's raw HID channel beyond
// what Oryx itself answers: to flash it without a key, to hear which layer
// it's on without hearing every key, to change layer, and to set its
// lights. ../host-commands.patch hands these over from ZSA's handler.
//
// Each command is a 32-byte report whose first byte says which; the ones
// past Oryx's own start at 0x80, clear of any it might add. A watched board
// answers with reports that open on HOST_WATCH: the layer it's on, then 1
// if that layer is locked and 0 if a key is holding it.

#include QMK_KEYBOARD_H
#include <LUFA/Drivers/USB/USB.h>
#include "keymap_introspection.h"
#include "oryx.h"
#include "usb_descriptor.h"

enum host_command {
    HOST_BOOTLOADER = 0x80,
    HOST_WATCH,
    HOST_LAYER,
    HOST_LIGHTS,
    HOST_BRIGHTNESS,
};

bool send_report(uint8_t endpoint, void *report, size_t size);

// Whether the computer is listening for layers, and what it was last told:
// the layer, with the top bit set while it's locked. Unlike pairing,
// watching says nothing about keys, so whatever is typed stays between the
// board and the keyboard's own interface.
static bool    watched = false;
static uint8_t told    = UINT8_MAX;

bool oryx_raw_hid_receive_user(uint8_t *data, uint8_t length) {
    uint8_t *param = &data[1];

    switch (data[0]) {
        // Oryx defines ending a pairing but never does it, and a paired
        // board waits on every key for a computer that may have gone.
        case ORYX_CMD_DISCONNECT:
            rawhid_state.paired = false;
            return true;

        case HOST_BOOTLOADER:
            reset_keyboard();
            return true;

        case HOST_WATCH:
            watched = param[0];
            told    = UINT8_MAX;
            return true;

        // A layer the computer moves to is locked there, as the layer lock
        // key does, so that key lets it go and the margin can tell it from
        // one a key is holding.
        case HOST_LAYER:
            if (param[0] < keymap_layer_count()) {
                layer_lock_all_off();
                layer_move(param[0]);
                if (param[0] > 0) {
                    layer_lock_on(param[0]);
                }
            }
            return true;

        // The lights' keys save what they set, but a computer can ask
        // often, so these last until the board restarts and spare its
        // EEPROM the writes.
        case HOST_LIGHTS:
            if (param[0]) {
                rgb_matrix_enable_noeeprom();
            } else {
                rgb_matrix_disable_noeeprom();
            }
            return true;

        case HOST_BRIGHTNESS:
            if (param[0]) {
                rgb_matrix_increase_val_noeeprom();
            } else {
                rgb_matrix_decrease_val_noeeprom();
            }
            return true;
    }
    return false;
}

// Read every pass of the main loop rather than from a layer change, so the
// keymap's own layer code stays as Oryx wrote it and a change made and undone
// within one pass, as some layer keys do, is never told at all.
void housekeeping_task_user(void) {
    if (!watched) {
        return;
    }
    uint8_t layer = get_highest_layer(layer_state);
    uint8_t now   = layer | (is_layer_locked(layer) ? 0x80 : 0);
    if (now == told) {
        return;
    }
    // A report still waiting is one the computer hasn't read, and writing
    // another would hold the board up to 10 ms for it, so this tries again
    // next pass. A computer that stops reading, asleep or gone, costs the
    // board nothing, and hears the layer as it is once it reads again.
    Endpoint_SelectEndpoint(RAW_IN_EPNUM);
    if (!Endpoint_IsReadWriteAllowed()) {
        return;
    }
    uint8_t report[RAW_EPSIZE] = {HOST_WATCH, layer, now >> 7};
    send_report(RAW_IN_EPNUM, report, sizeof(report));
    told = now;
}
