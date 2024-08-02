# waveforms-swd-protocol-parser

A hacked-together tool to parse SWD protocol operation logs from [DIGILENT WaveForms](https://digilent.com/shop/software/digilent-waveforms/)'s
SWD protocol analyzer.

I wrote this to figure out what SEGGER's tools were doing in order to add NXP S32K344 support
to [probe-rs](https://probe.rs/).

## Example

Given the following SWD log (`swd.log`):
```text
03:57:59.988:
DP WR A:0 ACK:1 OK Data:h0000001E 
DP WR A:2 ACK:1 OK Data:h04000000 
AP WR A:0 ACK:1 OK Data:h03000002 
AP WR A:1 ACK:1 OK Data:hE000EDF0 
DP WR A:2 ACK:2 Wait 
DP WR A:2 ACK:1 OK Data:h04000010 
AP RD A:0 ACK:1 OK Data:h40000000 
AP RD A:0 ACK:1 OK Data:h00030003 
DP WR A:0 ACK:1 OK Data:h0000001E 
DP WR A:2 ACK:1 OK Data:h04000000 
AP WR A:0 ACK:1 OK Data:h03000012 
AP WR A:1 ACK:1 OK Data:hE0001000 
AP RD A:3 ACK:2 Wait 
AP RD A:3 ACK:1 OK Data:h00030003 
DP RD A:3 ACK:1 OK Data:h40000001 
DP WR A:0 ACK:1 OK Data:h0000001E 
DP WR A:2 ACK:1 OK Data:h04000000 
AP WR A:0 ACK:1 OK Data:h03000012 
AP WR A:1 ACK:1 OK Data:hE0001004 
AP WR A:3 ACK:1 OK Data:h00000000 
DP RD A:3 ACK:1 OK Data:h40000001 
DP WR A:0 ACK:1 OK Data:h0000001E 
DP WR A:2 ACK:1 OK Data:h04000000 
AP WR A:0 ACK:1 OK Data:h03000002 
AP WR A:1 ACK:1 OK Data:hE000ED78 
AP RD A:3 ACK:2 Wait 
AP RD A:3 ACK:1 OK Data:h40000001 
AP RD A:3 ACK:1 OK Data:h09000003 
AP WR A:1 ACK:1 OK Data:hE000ED14 
AP RD A:3 ACK:2 Wait 
AP RD A:3 ACK:1 OK Data:h09000003 
AP RD A:3 ACK:1 OK Data:h00040200 
AP WR A:1 ACK:1 OK Data:hE000EF50 
AP WR A:3 ACK:1 OK Data:h00000000 
AP WR A:1 ACK:1 OK Data:hE000ED30 
AP WR A:3 ACK:1 OK Data:h0000001F 
AP WR A:1 ACK:1 OK Data:hE000EDF0 
DP WR A:2 ACK:2 Wait 
DP WR A:2 ACK:1 OK Data:h04000010 
AP RD A:0 ACK:1 OK Data:h00040200 
AP RD A:3 ACK:1 OK Data:h00030003 
AP RD A:3 ACK:1 OK Data:h01000000 
AP WR A:0 ACK:1 OK Data:hA05F0001 
DP RD A:3 ACK:1 OK Data:h01000000
```

Running `cargo run -- swd.log` produces the following:
```text
03:57:59.988:
DP WR A:0 ACK:1 OK Data:h0000001E  --> R:00 ABORT     DAPABORT:0 STKCMPCLR:1 STKERRCLR:1 WDERRCLR:1 ORUNERRCLR:1
DP WR A:2 ACK:1 OK Data:h04000000  --> R:08 SELECT    APSEL:04 APBANKSEL:00 CTRLSEL:0    (CM7_0_AHB_AP)
AP WR A:0 ACK:1 OK Data:h03000002  --> R:00 CSW       03000002
AP WR A:1 ACK:1 OK Data:hE000EDF0  --> R:04 TAR       E000EDF0
DP WR A:2 ACK:2 Wait
DP WR A:2 ACK:1 OK Data:h04000010  --> R:08 SELECT    APSEL:04 APBANKSEL:01 CTRLSEL:0    (CM7_0_AHB_AP)
AP RD A:0 ACK:1 OK Data:h40000000  <-- R:10 BD0       40000000    DHCSR (s_reset_st:0, s_halt:0, c_halt:0, c_debugen:0)
Dhcsr { .0: 1073741824, s_reset_st: false, s_retire_st: false, s_lockup: false, s_sleep: false, s_halt: false, s_regrdy: false, c_maskints: false, c_step: false, c_halt: false, c_debugen: false }
AP RD A:0 ACK:1 OK Data:h00030003  <-- R:10 BD0       00030003    DHCSR (s_reset_st:0, s_halt:1, c_halt:1, c_debugen:1)
Dhcsr { .0: 196611, s_reset_st: false, s_retire_st: false, s_lockup: false, s_sleep: false, s_halt: true, s_regrdy: true, c_maskints: false, c_step: false, c_halt: true, c_debugen: true }
DP WR A:0 ACK:1 OK Data:h0000001E  --> R:00 ABORT     DAPABORT:0 STKCMPCLR:1 STKERRCLR:1 WDERRCLR:1 ORUNERRCLR:1
DP WR A:2 ACK:1 OK Data:h04000000  --> R:08 SELECT    APSEL:04 APBANKSEL:00 CTRLSEL:0    (CM7_0_AHB_AP)
AP WR A:0 ACK:1 OK Data:h03000012  --> R:00 CSW       03000012
AP WR A:1 ACK:1 OK Data:hE0001000  --> R:04 TAR       E0001000
AP RD A:3 ACK:2 Wait
AP RD A:3 ACK:1 OK Data:h00030003  <-- R:0C DRW       00030003
DP RD A:3 ACK:1 OK Data:h40000001  <-- R:0C RDBUFF    40000001
DP WR A:0 ACK:1 OK Data:h0000001E  --> R:00 ABORT     DAPABORT:0 STKCMPCLR:1 STKERRCLR:1 WDERRCLR:1 ORUNERRCLR:1
DP WR A:2 ACK:1 OK Data:h04000000  --> R:08 SELECT    APSEL:04 APBANKSEL:00 CTRLSEL:0    (CM7_0_AHB_AP)
AP WR A:0 ACK:1 OK Data:h03000012  --> R:00 CSW       03000012
AP WR A:1 ACK:1 OK Data:hE0001004  --> R:04 TAR       E0001004
AP WR A:3 ACK:1 OK Data:h00000000  --> R:0C DRW       00000000
DP RD A:3 ACK:1 OK Data:h40000001  <-- R:0C RDBUFF    40000001
DP WR A:0 ACK:1 OK Data:h0000001E  --> R:00 ABORT     DAPABORT:0 STKCMPCLR:1 STKERRCLR:1 WDERRCLR:1 ORUNERRCLR:1
DP WR A:2 ACK:1 OK Data:h04000000  --> R:08 SELECT    APSEL:04 APBANKSEL:00 CTRLSEL:0    (CM7_0_AHB_AP)
AP WR A:0 ACK:1 OK Data:h03000002  --> R:00 CSW       03000002
AP WR A:1 ACK:1 OK Data:hE000ED78  --> R:04 TAR       E000ED78
AP RD A:3 ACK:2 Wait
AP RD A:3 ACK:1 OK Data:h40000001  <-- R:0C DRW       40000001
AP RD A:3 ACK:1 OK Data:h09000003  <-- R:0C DRW       09000003
AP WR A:1 ACK:1 OK Data:hE000ED14  --> R:04 TAR       E000ED14
AP RD A:3 ACK:2 Wait
AP RD A:3 ACK:1 OK Data:h09000003  <-- R:0C DRW       09000003
AP RD A:3 ACK:1 OK Data:h00040200  <-- R:0C DRW       00040200
AP WR A:1 ACK:1 OK Data:hE000EF50  --> R:04 TAR       E000EF50
AP WR A:3 ACK:1 OK Data:h00000000  --> R:0C DRW       00000000
AP WR A:1 ACK:1 OK Data:hE000ED30  --> R:04 TAR       E000ED30
AP WR A:3 ACK:1 OK Data:h0000001F  --> R:0C DRW       0000001F
AP WR A:1 ACK:1 OK Data:hE000EDF0  --> R:04 TAR       E000EDF0
DP WR A:2 ACK:2 Wait
DP WR A:2 ACK:1 OK Data:h04000010  --> R:08 SELECT    APSEL:04 APBANKSEL:01 CTRLSEL:0    (CM7_0_AHB_AP)
AP RD A:0 ACK:1 OK Data:h00040200  <-- R:10 BD0       00040200    DHCSR (s_reset_st:0, s_halt:0, c_halt:0, c_debugen:0)
Dhcsr { .0: 262656, s_reset_st: false, s_retire_st: false, s_lockup: false, s_sleep: true, s_halt: false, s_regrdy: false, c_maskints: false, c_step: false, c_halt: false, c_debugen: false }
AP RD A:3 ACK:1 OK Data:h00030003  <-- R:1C BD3       00030003
AP RD A:3 ACK:1 OK Data:h01000000  <-- R:1C BD3       01000000
AP WR A:0 ACK:1 OK Data:hA05F0001  --> R:10 BD0       A05F0001    DHCSR (s_reset_st:0, s_halt:1, c_halt:0, c_debugen:1)
Dhcsr { .0: 2690580481, s_reset_st: false, s_retire_st: false, s_lockup: true, s_sleep: true, s_halt: true, s_regrdy: true, c_maskints: false, c_step: false, c_halt: false, c_debugen: true }
DP RD A:3 ACK:1 OK Data:h01000000  <-- R:0C RDBUFF    01000000
---------------------------------------------------
Observed APs:
  4 (0x:04)
```
