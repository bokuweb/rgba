.equ VARBASE,	0x3000000
.equ TESTNUM,	(VARBASE+8)
.equ CURSEL,	(VARBASE+16)

.equ MENU, 	10

.equ BAD_Rd,	0x10
.equ BAD_Rn,	0x20    
    
    .arm
__start:
	stmfd sp!,{lr}
	
	ldr r0,=szALU1
	mov r1,#56
	mov r2,#1
	mov r3,#4

	@ ADC
	mov 	r1,#0
	mov 	r2,#0x80000000
	mov 	r3,#0xF
	adds 	r9,r9,r9	@ clear carry
	adcs 	r2,r2,r3
	orrcs 	r1,r1,#1
	orrpl 	r1,r1,#2
	orrvs 	r1,r1,#4
	orreq 	r1,r1,#8
	adcs 	r2,r2,r2	
	orrcc 	r1,r1,#1
	orrmi 	r1,r1,#2
	adc 	r3,r3,r3
	cmp 	r3,#0x1F
	orrne 	r1,r1,#BAD_Rd
	
	adds 	r9,r9,r9	@ clear carry
	mov 	r0,#0
	mov 	r2,#1
	adc 	r0,r0,r2,lsr#1
	cmp 	r0,#1
	@orrne 	r1,r1,#BAD_Rd
	
	ldr 	r0,=szADC

	add 	r8,r8,#8

	@ ADD
	mov 	r1,#0
	ldr 	r2,=0xFFFFFFFE
	mov 	r3,#1
	adds 	r2,r2,r3
	orrcs 	r1,r1,#1
	orrpl 	r1,r1,#2
	orrvs 	r1,r1,#4
	orreq 	r1,r1,#8
	adds 	r2,r2,r3	
	orrcc 	r1,r1,#1
	orrmi 	r1,r1,#2
	orrvs 	r1,r1,#4

	orrne 	r1,r1,#8
	ldr 	r0,=szADD
	add 	r8,r8,#8

	

	@ AND
	mov 	r1,#0
	mov 	r2,#2
	mov 	r3,#5
	ands 	r2,r2,r3,lsr#1
	orrcc 	r1,r1,#1
	orreq 	r1,r1,#8
	cmp 	r2,#2
	orrne 	r1,r1,#BAD_Rd
	mov 	r2,#0xC00
	mov 	r3,r2

	mov 	r4,#0x80000000
	ands 	r2,r2,r4,asr#32
	orrcc 	r1,r1,#1
	orrmi 	r1,r1,#2
	orreq 	r1,r1,#8
	cmp 	r2,r3
	orrne 	r1,r1,#BAD_Rd
	ldr 	r0,=szAND
	add 	r8,r8,#8


	@ BIC
	mov 	r1,#0
	adds 	r9,r9,r9 @ clear carry
	ldr 	r2,=0xFFFFFFFF
	ldr 	r3,=0xC000000D
	bics 	r2,r2,r3,asr#1
	orrcc 	r1,r1,#1
	orrmi 	r1,r1,#2	
	orreq 	r1,r1,#8
	ldr 	r3,=0x1FFFFFF9
	cmp 	r2,r3
	orrne 	r1,r1,#16
	ldr 	r0,=szBIC
	add 	r8,r8,#8
	
	@ CMN
	mov 	r1,#0
	adds 	r9,r9,r9 @ clear carry
	ldr 	r2,=0x7FFFFFFF
	ldr 	r3,=0x70000000
	cmn 	r2,r3
	orrcs 	r1,r1,#1
	orrpl 	r1,r1,#2
	orrvc 	r1,r1,#4
	orreq 	r1,r1,#8
	ldr 	r3,=0x7FFFFFFF
	cmp 	r2,r3
	orrne 	r1,r1,#16
	ldr 	r0,=szCMN
	add 	r8,r8,#8

	
	@ EOR
	mov 	r1,#0
	mov 	r2,#1
	mov 	r3,#3
	eors 	r2,r2,r3,lsl#31
	eors 	r2,r2,r3,lsl#0
	orrcc 	r1,r1,#1
	orrpl 	r1,r1,#2
	orreq 	r1,r1,#8
	ldr 	r4,=0x80000002
	cmp 	r4,r2
	orrne 	r1,r1,#BAD_Rd
	ldr 	r0,=szEOR
	add 	r8,r8,#8

	
	@ MOV
	mov 	r1,#0
	ldr 	r2,=labelone
	mov 	r3,r15
	cmp 	r2,r3
labelone:
	orrne 	r1,r1,#BAD_Rd
	ldr 	r2,=labeltwo
	mov 	r3,#0
	movs 	r4,r15,lsl r3	@ 0
	orreq 	r1,r1,#8	@ 4
	cmp 	r4,r2		@ 8
labeltwo: 			@ 12	
	orrne 	r1,r1,#BAD_Rd
	ldr 	r2,=0x80000001
	movs 	r3,r2,lsr#32
	orrcc 	r1,r1,#1
	orrmi 	r1,r1,#2
	
	orrne 	r1,r1,#8
	cmp 	r3,#0
	orrne 	r1,r1,#BAD_Rd

	@ Test ASR by reg==0
	mov 	r3,#3
	movs 	r4,r3,lsr#1	@ set carry 	
	mov 	r2,#0
	movs 	r3,r4,asr r2
	orrcc 	r1,r1,#1
	cmp 	r3,#1
	orrne 	r1,r1,#16

	@ Test ASR by reg==33
	ldr 	r2,=0x80000000
	mov 	r3,#33
	movs 	r2,r2,asr r3
	orrcc 	r1,r1,#1
	ldr 	r4,=0xFFFFFFFF
	cmp 	r2,r4
	orrne 	r1,r1,#16
	
	ldr 	r0,=szMOV
	add 	r8,r8,#8


	@ MVN
	mov 	r1,#0
	ldr 	r2,=labelthree	
	ldr 	r3,=0xFFFFFFFF
	eor 	r2,r2,r3
	mvn 	r3,r15
	cmp 	r3,r2
labelthree:	
	orrne 	r1,r1,#BAD_Rd
	ldr 	r0,=szMVN
	add 	r8,r8,#8
	
	@ ORR
	mov 	r1,#0
	mov 	r2,#2
	mov 	r3,#3
	movs 	r4,r3,lsr#1	@ set carry 
	orrs 	r3,r3,r2,rrx
	orrcs 	r1,r1,#1
	orrpl 	r1,r1,#2
	orreq 	r1,r1,#8
	ldr 	r4,=0x80000003
	cmp 	r4,r3
	orrne 	r1,r1,#BAD_Rd
	ldr 	r0,=szORR
	add 	r8,r8,#8
	
	
	@ RSC
	mov 	r1,#0
	mov 	r2,#2
	mov 	r3,#3
	adds 	r9,r9,r9	@ clear carry
	rscs 	r3,r2,r3
	orrcc 	r1,r1,#1
	orrmi 	r1,r1,#2
	orrne 	r1,r1,#8
	cmp 	r2,#2
	orrne 	r1,r1,#BAD_Rd
	ldr 	r0,=szRSC
	add 	r8,r8,#8

	
	@ SBC
	mov 	r1,#0
	ldr 	r2,=0xFFFFFFFF
	adds 	r3,r2,r2	@ set carry
	sbcs 	r2,r2,r2
	orrcc 	r1,r1,#1
	orrmi 	r1,r1,#2
	orrne 	r1,r1,#8
	adds 	r9,r9,r9	@ clear carry
	sbcs 	r2,r2,#0
	orreq 	r1,r1,#8
	orrcs 	r1,r1,#1
	orrpl 	r1,r1,#2
	ldr 	r0,=szSBC
	add 	r8,r8,#8
	

	@ MLA
	mov 	r1,#0
	ldr 	r2,=0xFFFFFFF6
	mov 	r3,#0x14
	ldr 	r4,=0xD0
	mlas 	r2,r3,r2,r4
	orrmi 	r1,r1,#2
	orreq 	r1,r1,#8
	cmp 	r2,#8
	orrne 	r1,r1,#16
	ldr 	r0,=szMLA
	add 	r8,r8,#8


	@ MUL
	mov 	r1,#0
	ldr 	r2,=0xFFFFFFF6
	mov 	r3,#0x14
	ldr 	r4,=0xFFFFFF38
	muls 	r2,r3,r2
	orrpl 	r1,r1,#2
	orreq 	r1,r1,#8
	cmp 	r2,r4
	orrne 	r1,r1,#16
	ldr 	r0,=szMUL
	add 	r8,r8,#8

	
	@ UMULL
	mov 	r1,#0
	ldr 	r2,=0x80000000
	mov 	r3,#8
	umulls 	r4,r5,r2,r3
	orrmi 	r1,r1,#2
	orreq 	r1,r1,#8
	mov 	r2,#4
	cmp 	r2,r5
	orrne 	r1,r1,#BAD_Rd
	ldr 	r0,=szUMULL
	add 	r8,r8,#8

	@ SMULL
	mov 	r1,#0
	ldr 	r2,=0x80000000
	mov 	r3,#8
	smulls 	r4,r5,r2,r3
	orrpl 	r1,r1,#2
	orreq 	r1,r1,#8
	ldr 	r2,=0xFFFFFFFC
	cmp 	r2,r5
	orrne 	r1,r1,#BAD_Rd
	ldr 	r0,=szSMULL
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
