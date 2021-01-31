.global main

.equ BAD_Rd,	0x10
.equ BAD_Rn,	0x20
    
    .arm
__start:
Test1:
	stmfd 	sp!,{lr}
	
	ldr 	r0,=szALU2
	mov 	r1,#60
	mov 	r2,#1
	mov 	r3,#4
	
	mov 	r1,#0
	ldr 	r2,=0x80000000
	mov 	r3,#8
	mov 	r5,#1
	mov 	r4,#2
	umlals 	r4,r5,r2,r3
	orrmi 	r1,r1,#2
	orreq 	r1,r1,#8
	cmp 	r4,#2
	orrne 	r1,r1,#BAD_Rd
	cmp 	r5,#5	
	orrne 	r1,r1,#BAD_Rd
	ldr 	r0,=szUMLAL
	add 	r8,r8,#8

	@ SMLAL
	mov 	r1,#0
	ldr 	r2,=0x80000001
	mov 	r3,#8
	mov 	r5,#5
	ldr 	r4,=0xfffffff8
	smlals 	r4,r5,r2,r3
	orrmi 	r1,r1,#2
	orreq 	r1,r1,#8
	cmp 	r5,#2
	orrne 	r1,r1,#BAD_Rd
	cmp 	r4,#0
	orrne 	r1,r1,#BAD_Rd
	ldr 	r0,=szSMLAL
	add 	r8,r8,#8

	@ MRS
	mov 	r1,#0
	mov 	r0,#0xC0000000
	adds 	r0,r0,r0		@ Z=0, C=1, V=0, N=1
	mov 	r2,#0x50000000
	mrs 	r2,cpsr
	tst 	r2,#0x20000000
	orreq 	r1,r1,#1
	tst 	r2,#0x80000000
	orreq 	r1,r1,#2
	tst 	r2,#0x10000000
	orrne 	r1,r1,#4
	tst 	r2,#0x40000000
	orrne 	r1,r1,#8
	ldr 	r0,=szMRS
	add 	r8,r8,#8
	
	
	@ MSR
	mov 	r1,#0
	movs 	r2,#0
	msr 	cpsr_flg,#0x90000000
	orrcs 	r1,r1,#1
	orrpl 	r1,r1,#2
	orrvc 	r1,r1,#4
	orreq 	r1,r1,#8
	mov 	r11,#1
	mrs 	r2,cpsr
	bic 	r2,r2,#0x1f
	orr 	r2,r2,#0x11	
	msr 	cpsr,r2		@ Set FIQ mode
	mov 	r11,#2
	orr 	r2,r2,#0x1f
	msr 	cpsr,r2		@ Set System mode
	cmp 	r11,#1
	orrne 	r1,r1,#BAD_Rd
	ldr 	r0,=szMSR
	add 	r8,r8,#8
	
	
	ldmfd 	sp!,{lr}
	mov 	pc,lr	
    ldr r0, =main
    bx r0
.pool
.align

.align 2
.global romvar
.global romvar2
.global romvar3
.global palette

romvar:  	.byte 0x80,0,0,0
romvar2: 	.byte 0x00,0x8f,0,0xff
romvar3: 	.byte 0x80,0x7f,0,0


.align 3
var64:		.word 0x11223344,0x55667788

rdVal:		.word 0
rnVal:		.word 0
memVal:		.word 0

palette:
		.hword 0x0000,0x0300,0x0018,0x7fff,0x7318,0x0E1F

menuitems:	.word szArm1,szArm2,szArm3
		.word szTmb1,szTmb2,szTmb3
		.word szArm4

menulinks:	.word 0,2,4
		.word 11,12,13
		.word 5
		

szADC:		.asciz "ADC"
szADD:		.asciz "ADD"
szAND:		.asciz "AND"
szBIC:		.asciz "BIC"
szCMN:		.asciz "CMN"
szCMP:		.asciz "CMP"
szEOR:		.asciz "EOR"
szMOV:		.asciz "MOV"
szMVN:		.asciz "MVN"
szORR:		.asciz "ORR"
szRSB:		.asciz "RSB"
szRSC:		.asciz "RSC"
szSBC:		.asciz "SBC"
szSUB:		.asciz "SUB"
szTEQ:		.asciz "TEQ"
szTST:		.asciz "TST"

szMLA:		.asciz "MLA"
szMUL:		.asciz "MUL"
szUMULL: 	.asciz "UMULL"
szUMLAL: 	.asciz "UMLAL"
szSMULL: 	.asciz "SMULL"
szSMLAL:	.asciz "SMLAL"

szSWP:		.asciz "SWP"
szSWPB:		.asciz "SWPB"

szMRS:		.asciz "MRS"
szMSR:		.asciz "MSR"

szLDR:		.asciz "LDR"
szLDRH:		.asciz "LDRH"
szLDRB:		.asciz "LDRB"
szLDRSH:	.asciz "LDRSH"
szLDRSB:	.asciz "LDRSB"
szSTR:		.asciz "STR"
szSTRH:		.asciz "STRH"
szSTRB:		.asciz "STRB"

szLDM:		.asciz "LDM"
szSTM:		.asciz "STM"

szCLZ:		.asciz "CLZ"
szLDRD:		.asciz "LDRD"
szMRC:		.asciz "MRC"
szQADD:		.asciz "QADD"
szSMLABB:	.asciz "SMLABB"
szSMLABT:	.asciz "SMLABT"
szSMLATB:	.asciz "SMLATB"
szSMLATT:	.asciz "SMLATT"
szSMULBB:	.asciz "SMULBB"
szSMULBT:	.asciz "SMULBT"
szSMULTB:	.asciz "SMULTB"
szSMULTT:	.asciz "SMULTT"


.global szLDRtype
szLDRtype:	.ascii "&\\] "
		.byte 0
	  	.ascii "'\\] "
		.byte 0
	  	.ascii "&\\])"
		.byte 0
	  	.ascii "'\\])"
		.byte 0
	  	.ascii "&R] "
		.byte 0
	  	.ascii "'R] "
		.byte 0
	  	.ascii "&R])"
		.byte 0
	  	.ascii "'R])"
		.byte 0
	  	.ascii "]&\\ "
		.byte 0
	  	.ascii "]'\\ "
		.byte 0
	  	.ascii "]&R "
		.byte 0
	  	.ascii "]'R "
		.byte 0

szLDMtype:	.ascii "IB) "
		.byte 0
		.ascii "IA) "
		.byte 0
		.ascii "DB) "
		.byte 0
		.ascii "DA) "
		.byte 0
		.ascii "IBS)"
		.byte 0
		.ascii "IAS)"
		.byte 0
		.ascii "DBS)"
		.byte 0
		.ascii "DBA)"
		.byte 0
		
szOK:		.asciz "OK"
szBad:		.asciz "BAD"
szRd:		.asciz "R^"
szRn:		.asciz "R_"
szC:		.asciz "C"
szN:		.asciz "N"
szV:		.asciz "V"
szZ:		.asciz "Z"
szQ:		.asciz "Q"
szSel:		.asciz "`"
szMem:		.asciz "MEM"

szAsterixes:	.asciz "jjjjjjjjjjjjjjjjjjjjjjjjjjjjjj"
szAster2:	    .asciz "j                            j"
szArmwrestler:	.asciz "j      ARMWRESTLER GBA       j"
szAuthor:	    .asciz "j         Micg 2006          j"
szAuthor2:	    .asciz "j       NORMMATTg 2012       j"

szArm1:		.asciz "ARM ALU"
szArm2:		.asciz "ARM LDR/STR"
szArm3:		.asciz "ARM LDM/STM"
szTmb1:		.asciz "THUMB ALU"
szTmb2:		.asciz "THUMB LDR/STR"
szTmb3:		.asciz "THUMB LDM/STM"
szArm4:		.asciz "ARM V5TE"
szMarker:	.asciz "`"
szSelect:	.asciz "SELECT A TEST AND PRESS"

szALU1:		.asciz  "ALU TESTS PART 1"
szALU2:		.asciz  "ALU PT 2 / MISC"
szLS1: 		.asciz   "LOAD TESTS PART 1"
szLS2: 		.asciz   "LOAD TESTS PART 2"
szLS3: 		.asciz   "LOAD TESTS PART 3"
szLS4: 		.asciz   "LOAD TESTS PART 4"
szLS5: 		.asciz   "LOAD TESTS PART 5"
szLS6: 		.asciz   "LOAD TESTS PART 6"
szLS7: 		.asciz   "LOAD TESTS PART 7"
szLS8: 		.asciz   "LOAD TESTS PART 8"
szLDM1:		.asciz 	"LDM/STM TESTS 1"
szAV5:		.asciz  "ARM V5TE TESTS"

.global szSelect2
.global szNext
.global szStart
.global szMenu

szPressStart: 	.asciz "PRESS START"
szSelect2:	.asciz "SELECT:"
szNext:		.asciz "Nextg"
szStart:	.asciz "START:"
szStart2:	.asciz "START"
szMenu:		.asciz "Menu"
szSpace:	.asciz " "

szRomMsg:	.asciz "Message from ROM"

szHexNum:	.asciz "00000000"

.align
.pool
@.end
