#include QMK_KEYBOARD_H
#include "version.h"

enum custom_keycodes {
  RGB_SLD = EZ_SAFE_RANGE,
  MAC_DND,
  MAC_LOCK,
};



const uint16_t PROGMEM keymaps[][MATRIX_ROWS][MATRIX_COLS] = {
  [0] = LAYOUT_ergodox_pretty(
    KC_ESCAPE,      KC_F1,          KC_F2,          KC_F3,          KC_F4,          KC_F5,          KC_MEH,                                         KC_MEH,         KC_F6,          KC_F7,          KC_F8,          KC_F9,          KC_F10,         KC_ESCAPE,
    KC_TAB,         KC_Q,           KC_W,           KC_F,           KC_P,           KC_B,           KC_HOME,                                        KC_PAGE_UP,     KC_J,           KC_L,           KC_U,           KC_Y,           KC_APPLICATION, KC_TAB,
    KC_LEFT_SHIFT,  MT(MOD_LCTL, KC_A),MT(MOD_LALT, KC_R),LT(1, KC_S),    MT(MOD_LGUI, KC_T),LT(2, KC_G),                                                                    LT(2, KC_M),    MT(MOD_RGUI, KC_N),LT(1, KC_E),    MT(MOD_RALT, KC_I),MT(MOD_RCTL, KC_O),KC_RIGHT_SHIFT,
    KC_ENTER,       KC_Z,           KC_X,           KC_C,           KC_D,           KC_V,           KC_END,                                         KC_PGDN,        KC_K,           KC_H,           KC_UP,          KC_NO,          KC_NO,          KC_ENTER,
    KC_AUDIO_VOL_DOWN,KC_AUDIO_VOL_UP,KC_MEDIA_PREV_TRACK,KC_MEDIA_PLAY_PAUSE,KC_MEDIA_NEXT_TRACK,                                                                                                KC_LEFT,        KC_DOWN,        KC_RIGHT,       KC_BRIGHTNESS_DOWN,KC_BRIGHTNESS_UP,
                                                                                                    CW_TOGG,        KC_LEFT_GUI,    KC_RIGHT_GUI,   CW_TOGG,
                                                                                                                    KC_INSERT,      KC_INSERT,
                                                                                    KC_SPACE,       KC_BSPC,        KC_DELETE,      KC_DELETE,      KC_BSPC,        KC_SPACE
  ),
  [1] = LAYOUT_ergodox_pretty(
    KC_TRANSPARENT, KC_TRANSPARENT, KC_TRANSPARENT, KC_TRANSPARENT, KC_TRANSPARENT, KC_TRANSPARENT, KC_TRANSPARENT,                                 KC_TRANSPARENT, KC_TRANSPARENT, KC_TRANSPARENT, KC_TRANSPARENT, KC_TRANSPARENT, KC_TRANSPARENT, KC_F11,
    KC_TRANSPARENT, KC_PLUS,        KC_PIPE,        KC_LBRC,        KC_CIRC,        KC_EXLM,        KC_TRANSPARENT,                                 KC_TRANSPARENT, KC_QUES,        KC_DLR,         KC_RBRC,        KC_EQUAL,       KC_ASTR,        KC_F12,
    KC_TRANSPARENT, KC_LABK,        KC_LCBR,        KC_TRANSPARENT, KC_QUOTE,       KC_GRAVE,                                                                       KC_TILD,        KC_DQUO,        KC_TRANSPARENT, KC_RCBR,        KC_RABK,        KC_TRANSPARENT,
    KC_TRANSPARENT, KC_BSLS,        KC_AT,          KC_COMMA,       KC_MINUS,       KC_HASH,        KC_TRANSPARENT,                                 KC_TRANSPARENT, KC_PERC,        KC_UNDS,        KC_DOT,         KC_AMPR,        KC_SLASH,       KC_TRANSPARENT,
    KC_TRANSPARENT, KC_TRANSPARENT, KC_TRANSPARENT, KC_TRANSPARENT, KC_TRANSPARENT,                                                                                                 KC_TRANSPARENT, KC_TRANSPARENT, KC_TRANSPARENT, KC_TRANSPARENT, KC_TRANSPARENT,
                                                                                                    KC_TRANSPARENT, KC_TRANSPARENT, KC_TRANSPARENT, KC_TRANSPARENT,
                                                                                                                    KC_TRANSPARENT, KC_TRANSPARENT,
                                                                                    KC_LPRN,        KC_SCLN,        KC_TRANSPARENT, KC_TRANSPARENT, KC_COLN,        KC_RPRN
  ),
  [2] = LAYOUT_ergodox_pretty(
    KC_SYSTEM_POWER,KC_TRANSPARENT, KC_TRANSPARENT, KC_TRANSPARENT, KC_TRANSPARENT, KC_TRANSPARENT, KC_TRANSPARENT,                                 KC_TRANSPARENT, KC_TRANSPARENT, KC_TRANSPARENT, KC_TRANSPARENT, KC_TRANSPARENT, KC_TRANSPARENT, KC_SYSTEM_POWER,
    MAC_LOCK,       RGB_SPD,        RGB_SPI,        RGB_MODE_FORWARD,KC_TRANSPARENT, KC_TRANSPARENT, KC_TRANSPARENT,                                 KC_TRANSPARENT, KC_TRANSPARENT, KC_KP_PLUS,     KC_KP_7,        KC_KP_8,        KC_KP_9,        MAC_LOCK,
    MAC_DND,        RGB_HUD,        RGB_HUI,        KC_TRANSPARENT, KC_TRANSPARENT, KC_TRANSPARENT,                                                                 KC_TRANSPARENT, KC_KP_MINUS,    KC_KP_4,        KC_KP_5,        KC_KP_6,        MAC_DND,
    KC_TRANSPARENT, RGB_SAD,        RGB_SAI,        KC_TRANSPARENT, KC_TRANSPARENT, KC_TRANSPARENT, KC_TRANSPARENT,                                 KC_TRANSPARENT, KC_TRANSPARENT, KC_KP_ASTERISK, KC_KP_1,        KC_KP_2,        KC_KP_3,        KC_TRANSPARENT,
    KC_TRANSPARENT, KC_TRANSPARENT, KC_TRANSPARENT, KC_TRANSPARENT, KC_TRANSPARENT,                                                                                                 KC_KP_SLASH,    KC_KP_0,        KC_KP_DOT,      KC_KP_ENTER,    KC_TRANSPARENT,
                                                                                                    KC_TRANSPARENT, QK_LLCK,        QK_LLCK,        KC_TRANSPARENT,
                                                                                                                    KC_PSCR,        KC_PSCR,
                                                                                    KC_TRANSPARENT, KC_TRANSPARENT, QK_BOOT,        QK_BOOT,        KC_TRANSPARENT, KC_TRANSPARENT
  ),
};




extern rgb_config_t rgb_matrix_config;

void keyboard_post_init_user(void) {
  rgb_matrix_enable();
}

const uint8_t PROGMEM ledmap[][RGB_MATRIX_LED_COUNT][3] = {
    [1] = { {0,0,0}, {0,0,0}, {0,0,0}, {0,0,0}, {0,0,0}, {0,0,0}, {0,0,0}, {0,0,0}, {0,0,0}, {0,0,0}, {0,0,0}, {0,0,0}, {0,0,0}, {0,0,0}, {0,0,0}, {0,0,0}, {0,0,0}, {0,0,0}, {0,0,0}, {0,0,0}, {0,0,0}, {0,0,0}, {0,0,0}, {0,0,0}, {0,0,0}, {0,0,0}, {0,0,0}, {0,0,0}, {0,0,0}, {0,0,0}, {0,0,0}, {0,0,0}, {0,0,0}, {0,0,0}, {0,0,0}, {0,0,0}, {0,0,0}, {0,0,0}, {0,0,0}, {0,0,0}, {0,0,0}, {0,0,0}, {0,0,0}, {0,0,0}, {0,0,0}, {0,0,0}, {0,0,0}, {0,0,0} },

};

void set_layer_color(int layer) {
  for (int i = 0; i < RGB_MATRIX_LED_COUNT; i++) {
    HSV hsv = {
      .h = pgm_read_byte(&ledmap[layer][i][0]),
      .s = pgm_read_byte(&ledmap[layer][i][1]),
      .v = pgm_read_byte(&ledmap[layer][i][2]),
    };
    if (!hsv.h && !hsv.s && !hsv.v) {
        rgb_matrix_set_color( i, 0, 0, 0 );
    } else {
        RGB rgb = hsv_to_rgb( hsv );
        float f = (float)rgb_matrix_config.hsv.v / UINT8_MAX;
        rgb_matrix_set_color( i, f * rgb.r, f * rgb.g, f * rgb.b );
    }
  }
}

bool rgb_matrix_indicators_user(void) {
  if (keyboard_config.disable_layer_led) { return false; }
  switch (biton32(layer_state)) {
    case 1:
      set_layer_color(1);
      break;
   default:
    if (rgb_matrix_get_flags() == LED_FLAG_NONE)
      rgb_matrix_set_color_all(0, 0, 0);
    break;
  }
  return true;
}

bool process_record_user(uint16_t keycode, keyrecord_t *record) {
  switch (keycode) {
    case MAC_DND:
      HSS(0x9B);
    case MAC_LOCK:
      HCS(0x19E);

    case RGB_SLD:
      if (record->event.pressed) {
        rgblight_mode(1);
      }
      return false;
  }
  return true;
}

uint8_t layer_state_set_user(uint8_t state) {
    uint8_t layer = biton(state);
  ergodox_board_led_off();
  ergodox_right_led_1_off();
  ergodox_right_led_2_off();
  ergodox_right_led_3_off();
  switch (layer) {
    case 1:
      ergodox_right_led_1_on();
      break;
    case 2:
      ergodox_right_led_2_on();
      break;
    case 3:
      ergodox_right_led_3_on();
      break;
    case 4:
      ergodox_right_led_1_on();
      ergodox_right_led_2_on();
      break;
    case 5:
      ergodox_right_led_1_on();
      ergodox_right_led_3_on();
      break;
    case 6:
      ergodox_right_led_2_on();
      ergodox_right_led_3_on();
      break;
    case 7:
      ergodox_right_led_1_on();
      ergodox_right_led_2_on();
      ergodox_right_led_3_on();
      break;
    default:
      break;
  }
  return state;
};



