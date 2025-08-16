# Hardware timers

This page describes supported hardware timers, for each architecture.

## x86_64 and x86

| Name | Notes                                                                                                       |
|------|-------------------------------------------------------------------------------------------------------------|
| PIT  | Used only when the APIC is not present                                                                      |
| RTC  | Currently used for timekeeping. It shall later be replaced by something else and be used only as a fallback |
| APIC | Used to drive the scheduler                                                                                 |
| HPET | Used to calibrate the APIC timer                                                                            |

The frequency of the **APIC timer** is not known, thus we need to **calibrate** it (determine its frequency).
This is done at boot using the **HPET**.