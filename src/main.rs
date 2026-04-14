#![no_std]
#![no_main]
use embassy_executor::Spawner;
use embassy_time::Timer;
use embassy_rp::{
    adc::{Adc, Channel, Config as AdcConfig, InterruptHandler},
    bind_interrupts,
    gpio::{Level, Output, Pull},
    pwm::{self, Config as PwmConfig},
};
use panic_probe as _;
use defmt_rtt as _;

bind_interrupts!(struct Irqs {
    ADC_IRQ_FIFO => InterruptHandler;
});

const COUNTS_PER_SEMITONE: u16 = 103;

fn quantize_to_scale(value: u16, scale: &[u8]) -> u16 {
    let total_semitones = value / COUNTS_PER_SEMITONE;
    let octave = total_semitones / 12;
    let note = total_semitones % 12;
    let mut closest = scale[0];
    let mut curr_min_distance = 12u16;
    for &scale_note in scale {
        let distance = note.abs_diff(scale_note as u16);
        if distance < curr_min_distance {
            curr_min_distance = distance;
            closest = scale_note;
        }
    }
    (octave * 12 + closest as u16) * COUNTS_PER_SEMITONE
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_rp::init(Default::default());
    let mut adc = Adc::new(p.ADC, Irqs, AdcConfig::default());
    let mut potent_1 = Channel::new_pin(p.PIN_26, Pull::None);
    let mut led = Output::new(p.PIN_15, Level::Low);

    let mut pwm_config = PwmConfig::default();
    pwm_config.top = 2082;
    pwm_config.divider = 1u8.into();
    let mut pwm_out = pwm::Pwm::new_output_a(p.PWM_SLICE5, p.PIN_10, pwm_config.clone());

    const MAJOR: &[u8] = &[0, 2, 4, 5, 7, 9, 11];

    loop {
        let Ok(value) = adc.read(&mut potent_1).await else {
            continue;
        };
        let quantized = quantize_to_scale(value, MAJOR);

        // PWM output
        let duty = (quantized as u32 * 2083 / 4096) as u16;
        pwm_config.compare_a = duty;
        pwm_out.set_config(&pwm_config);

        // debug — blink note+1 times
        //let total_semitones = quantized / COUNTS_PER_SEMITONE;
        //let note = total_semitones % 12;
        //for _ in 0..=note {
        //    led.set_high();
        //    Timer::after_millis(100).await;
        //    led.set_low();
        //    Timer::after_millis(100).await;
        //}
        //Timer::after_millis(500).await;
        Timer::after_micros(100).await;
    }
}
