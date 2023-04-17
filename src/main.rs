#![no_std]
#![no_main]

use esp_backtrace as _;
use esp_println::println;
use hal::{
    clock::ClockControl, peripherals::Peripherals, prelude::*, timer::TimerGroup, Delay, Rtc,
};
#[entry]
fn main() -> ! {
    let peripherals = Peripherals::take();
    let system = peripherals.SYSTEM.split();
    let clocks = ClockControl::boot_defaults(system.clock_control).freeze();

    // Disable the RTC and TIMG watchdog timers
    let mut rtc = Rtc::new(peripherals.RTC_CNTL);
    let timer_group0 = TimerGroup::new(peripherals.TIMG0, &clocks);
    let mut wdt0 = timer_group0.wdt;
    let timer_group1 = TimerGroup::new(peripherals.TIMG1, &clocks);
    let mut wdt1 = timer_group1.wdt;
    rtc.swd.disable();
    rtc.rwdt.disable();
    wdt0.disable();
    wdt1.disable();

    println!(
        "Hello world! rtc_time = {} reset-reason= {:?}",
        get_rtc_time(),
        hal::rtc_cntl::get_reset_reason(hal::Cpu::ProCpu)
    );

    let mut delay = Delay::new(&clocks);
    delay.delay_ms(500 as u32);

    let sleep_ticks: u64 = 5_000_000 + get_rtc_time();
    unsafe {
        let rtc = &*pac::RTC_CNTL::PTR;

        rtc.dig_pwc.modify(|_, w| w.dg_wrap_pd_en().set_bit()); // enable power down digital core in sleep

        rtc.wakeup_state.modify(|_, w| w.wakeup_ena().variant(0x8)); // 0x08 = RTC timer wakeup

        rtc.int_clr_rtc.write(|w| {
            w.slp_reject_int_clr()
                .set_bit()
                .slp_wakeup_int_clr()
                .set_bit()
        });

        rtc.slp_timer0
            .write(|w| w.bits((sleep_ticks & u32::MAX as u64) as u32));
        rtc.slp_timer1.modify(|_, w| {
            w.slp_val_hi()
                .variant(((sleep_ticks >> 32) & u16::MAX as u64) as u16)
        });

        rtc.slp_timer1
            .modify(|_, w| w.main_timer_alarm_en().set_bit());
        rtc.int_clr_rtc.write(|w| w.bits(1 << 8));
        rtc.state0.modify(|_, w| w.sleep_en().set_bit());
    }

    // in deep-sleep we won't end up here
    println!("Hello world!");
    println!("{}", get_rtc_time());

    loop {}
}

fn get_rtc_time() -> u64 {
    unsafe {
        let rtc = &*pac::RTC_CNTL::PTR;
        rtc.slp_timer0.read().bits() as u64
            | (rtc.slp_timer1.read().slp_val_hi().bits() as u64) << 32
    }
}
