.equ VARBASE,	0x3000000
.equ TESTNUM,	(VARBASE+8)
.equ CURSEL,	(VARBASE+16)

.equ MENU, 	10

.equ BAD_Rd,	0x10
.equ BAD_Rn,	0x20    
    
    .arm
__start:

	stmfd 	sp!,{lr}

	ldr 	r0,=szLS1
	mov 	r1,#52
	mov 	r2,#1
	mov 	r3,#4


	@ LDR
	
	@ +#]
	mov 	r1,#0
	ldr 	r0,=romvar
	sub 	r2,r0,#3
	mov 	r3,r2
	ldr 	r0,[r0,#0]
	cmp 	r0,#0x80
	orrne 	r1,r1,#BAD_Rd
	ldr 	r0,[r2,#3]
	cmp 	r0,#0x80
	orrne 	r1,r1,#BAD_Rd
	cmp 	r2,r3
	orrne 	r1,r1,#BAD_Rn
	@ Test non word-aligned load
	ldr 	r0,=romvar2
	ldr 	r0,[r0,#1]
	ldr 	r2,=0x00ff008f
	cmp 	r0,r2
	orrne 	r1,r1,#BAD_Rd
	ldr 	r0,=romvar2
	ldr 	r0,[r0,#2]
	ldr 	r2,=0x8f00ff00
	cmp 	r0,r2
	orrne 	r1,r1,#BAD_Rd
	ldr 	r0,=romvar2
	ldr 	r0,[r0,#3]
	ldr 	r2,=0x008f00ff
	cmp 	r0,r2
	orrne 	r1,r1,#BAD_Rd

	mov 	r2,#0
	orr 	r1,r1,#0x80000000
	ldr 	r0,=szLDR
	add 	r8,r8,#8


	@ -#]
	mov 	r1,#0
	ldr 	r0,=romvar
	mov 	r2,r0
	mov 	r3,r2
	add 	r0,r0,#206
	ldr 	r0,[r0,#-206]
	cmp 	r0,#0x80
	orrne 	r1,r1,#BAD_Rd
	ldr 	r0,[r2,#-0]
	cmp 	r0,#0x80
	orrne 	r1,r1,#BAD_Rd
	cmp 	r2,r3
	orrne 	r1,r1,#BAD_Rn
	@ Test non word-aligned load
	ldr 	r0,=romvar2+4
	ldr 	r0,[r0,#-2]
	ldr 	r2,=0x8f00ff00
	cmp 	r0,r2
	orrne 	r1,r1,#BAD_Rd

	mov 	r2,#1
	orr 	r1,r1,#0x80000000
	ldr 	r0,=szLDR
	add 	r8,r8,#8


	@ +#]!
	mov 	r1,#0
	ldr 	r0,=romvar
	sub 	r2,r0,#3
	mov 	r3,r0
	ldr 	r0,[r0,#0]!
	cmp 	r0,#0x80
	orrne 	r1,r1,#BAD_Rd
	ldr 	r0,[r2,#3]!
	cmp 	r0,#0x80
	orrne 	r1,r1,#BAD_Rd
	cmp 	r2,r3
	orrne 	r1,r1,#BAD_Rn
	@ Test non word-aligned load
	ldr 	r0,=romvar2
	ldr 	r0,[r0,#2]!
	ldr 	r2,=0x8f00ff00
	cmp 	r0,r2
	orrne 	r1,r1,#BAD_Rd
	
	mov 	r2,#2
	orr 	r1,r1,#0x80000000
	ldr 	r0,=szLDR
	add 	r8,r8,#8


	@ -#]!
	mov 	r1,#0
	ldr 	r0,=romvar
	add 	r2,r0,#1
	mov 	r3,r0
	add 	r0,r0,#206
	ldr 	r0,[r0,#-206]!
	cmp 	r0,#0x80
	orrne 	r1,r1,#BAD_Rd
	ldr 	r0,[r2,#-1]!
	cmp 	r0,#0x80
	orrne 	r1,r1,#BAD_Rd
	cmp 	r2,r3
	orrne 	r1,r1,#BAD_Rn
	@ Test non word-aligned load
	ldr 	r0,=romvar2+4
	ldr 	r0,[r0,#-2]!
	ldr 	r2,=0x8f00ff00
	cmp 	r0,r2
	orrne 	r1,r1,#BAD_Rd
	
	mov 	r2,#3
	orr 	r1,r1,#0x80000000
	ldr 	r0,=szLDR
	add 	r8,r8,#8


	@ +R]
	mov 	r1,#0
	ldr 	r0,=romvar
	sub 	r2,r0,#8
	sub 	r0,r0,#1
	mov 	r3,r2
	mov 	r4,#2
	ldr 	r0,[r0,r4, lsr #1]
	cmp 	r0,#0x80
	orrne 	r1,r1,#BAD_Rd
	ldr 	r0,[r2,r4, lsl #2]
	cmp 	r0,#0x80
	orrne 	r1,r1,#BAD_Rd
	cmp 	r2,r3
	orrne 	r1,r1,#BAD_Rn

	ldr 	r2,=romvar
	mov 	r2,r2,lsr#1
	mov 	r3,#0xC0000000
	ldr 	r0,[r2,r2]
	cmp 	r0,#0x80
	orrne 	r1,r1,#BAD_Rd

	ldr 	r2,=romvar
	mov 	r3,#0x8
	ldr 	r0,[r2,r3, lsr #32]
	cmp 	r0,#0x80
	orrne 	r1,r1,#BAD_Rd
	
	ldr 	r2,=romvar
	add 	r2,r2,#1
	mov 	r3,#0xC0000000
	ldr 	r0,[r2,r3, asr #32]
	cmp 	r0,#0x80
	orrne 	r1,r1,#BAD_Rd

	ldr 	r2,=romvar
	add 	r2,r2,#2
	ldr 	r3,=0xfffffffc
	adds 	r4,r3,r3		@ set carry
	ldr 	r0,[r2,r3, rrx]
	orrcc 	r1,r1,#1
	cmp 	r0,#0x80
	orrne 	r1,r1,#BAD_Rd

	@ Test non word-aligned load
	ldr 	r0,=romvar2
	mov 	r2,#2
	ldr 	r0,[r0,r2]
	ldr 	r2,=0x8f00ff00
	cmp 	r0,r2
	orrne 	r1,r1,#BAD_Rd
	
	mov 	r2,#4
	orr 	r1,r1,#0x80000000
	ldr 	r0,=szLDR
	add 	r8,r8,#8


	@ -R]
	mov 	r1,#0
	ldr 	r0,=romvar
	add 	r2,r0,#8
	add 	r0,r0,#1
	mov 	r3,r2
	mov 	r4,#2
	ldr 	r0,[r0,-r4, lsr #1]
	cmp 	r0,#0x80
	orrne 	r1,r1,#BAD_Rd
	ldr 	r0,[r2,-r4, lsl #2]
	cmp 	r0,#0x80
	orrne 	r1,r1,#BAD_Rd
	cmp 	r2,r3
	orrne 	r1,r1,#BAD_Rn

	ldr 	r2,=romvar
	mov 	r3,#0x8
	ldr 	r0,[r2,-r3, lsr #32]
	cmp 	r0,#0x80
	orrne 	r1,r1,#BAD_Rd

	ldr 	r2,=romvar
	sub 	r2,r2,#1
	mov 	r3,#0x80000000
	ldr 	r0,[r2,-r3, asr #32]
	cmp 	r0,#0x80
	orrne 	r1,r1,#BAD_Rd
	
	ldr 	r2,=romvar
	sub 	r2,r2,#4
	ldr 	r3,=0xfffffff8
	adds 	r4,r3,r3		@ set carry
	ldr 	r0,[r2,-r3, rrx]
	orrcc 	r1,r1,#1
	cmp 	r0,#0x80
	orrne 	r1,r1,#BAD_Rd

	@ Test non word-aligned load
	ldr 	r0,=romvar2+4
	mov 	r2,#1
	ldr 	r0,[r0,-r2, lsl #1]
	ldr 	r2,=0x8f00ff00
	cmp 	r0,r2
	orrne 	r1,r1,#BAD_Rd
	
	mov 	r2,#5
	orr 	r1,r1,#0x80000000
	ldr 	r0,=szLDR
	add 	r8,r8,#8
	

	@ +R]!
	mov 	r1,#0
	ldr 	r0,=romvar
	mov 	r3,r0
	sub 	r2,r0,#8
	sub 	r0,r0,#1
	mov 	r4,#2
	ldr 	r0,[r0,r4, lsr #1]!
	cmp 	r0,#0x80
	orrne 	r1,r1,#BAD_Rd
	ldr 	r0,[r2,r4, lsl #2]!
	cmp 	r0,#0x80
	orrne 	r1,r1,#BAD_Rd
	cmp 	r2,r3
	orrne 	r1,r1,#BAD_Rn

	ldr 	r2,=romvar
	mov 	r4,r2
	mov 	r2,r2,lsr#1
	mov 	r3,#0xC0000000
	ldr 	r0,[r2,r2]!
	cmp 	r0,#0x80
	orrne 	r1,r1,#BAD_Rd
	cmp 	r2,r4
	orrne 	r1,r1,#BAD_Rn
	
	ldr 	r2,=romvar
	mov 	r4,r2
	add 	r2,r2,#1
	mov 	r3,#0xC0000000
	ldr 	r0,[r2,r3, asr #32]!
	cmp 	r0,#0x80
	orrne 	r1,r1,#BAD_Rd
	cmp 	r2,r4
	orrne 	r1,r1,#BAD_Rn

	ldr 	r2,=romvar
	mov 	r5,r2
	add 	r2,r2,#2
	ldr 	r3,=0xfffffffc
	adds 	r4,r3,r3		@ set carry
	ldr 	r0,[r2,r3, rrx]!
	orrcc 	r1,r1,#1
	cmp 	r0,#0x80
	orrne 	r1,r1,#BAD_Rd
	cmp 	r2,r5
	orrne 	r1,r1,#BAD_Rn

	@ Test non word-aligned load
	ldr 	r0,=romvar2
	mov 	r2,#2
	ldr 	r0,[r0,r2]!
	ldr 	r2,=0x8f00ff00
	cmp 	r0,r2
	orrne 	r1,r1,#BAD_Rd
	
	mov 	r2,#6
	orr 	r1,r1,#0x80000000
	ldr 	r0,=szLDR
	add 	r8,r8,#8


	@ -R]!
	mov 	r1,#0
	ldr 	r0,=romvar
	mov 	r3,r0
	add 	r2,r0,#8
	add 	r0,r0,#1
	mov 	r4,#2
	ldr 	r0,[r0,-r4, lsr #1]!
	cmp 	r0,#0x80
	orrne 	r1,r1,#BAD_Rd
	ldr 	r0,[r2,-r4, lsl #2]!
	cmp 	r0,#0x80
	orrne 	r1,r1,#BAD_Rd
	cmp 	r2,r3
	orrne 	r1,r1,#BAD_Rn

	ldr 	r2,=romvar
	mov 	r4,r2
	sub 	r2,r2,#1
	mov 	r3,#0x80000000
	ldr 	r0,[r2,-r3, asr #32]!
	cmp 	r0,#0x80
	orrne 	r1,r1,#BAD_Rd
	cmp 	r2,r4
	orrne 	r1,r1,#BAD_Rn
	
	ldr 	r2,=romvar
	mov 	r5,r2
	sub 	r2,r2,#4
	ldr 	r3,=0xfffffff8
	adds 	r4,r3,r3		@ set carry
	ldr 	r0,[r2,-r3, rrx]!
	orrcc 	r1,r1,#1
	cmp 	r0,#0x80
	orrne 	r1,r1,#BAD_Rd
	cmp 	r2,r5
	orrne 	r1,r1,#BAD_Rn

	@ Test non word-aligned load
	ldr 	r0,=romvar2+4
	mov 	r2,#2
	ldr 	r0,[r0,-r2]!
	ldr 	r2,=0x8f00ff00
	cmp 	r0,r2
	orrne 	r1,r1,#BAD_Rd
	
	mov 	r2,#7
	orr 	r1,r1,#0x80000000
	ldr 	r0,=szLDR
	add 	r8,r8,#8
	

	@ ]+#
	mov 	r1,#0
	ldr 	r0,=romvar
	add 	r3,r0,#3
	mov 	r2,r0
	ldr 	r0,[r0],#3
	cmp 	r0,#0x80
	orrne 	r1,r1,#BAD_Rd
	ldr 	r0,[r2],#3
	cmp 	r0,#0x80
	orrne 	r1,r1,#BAD_Rd
	cmp 	r2,r3
	orrne 	r1,r1,#BAD_Rn
	@ Test non word-aligned load
	ldr 	r0,=romvar2+2
	ldr 	r0,[r0],#5
	ldr 	r2,=0x8f00ff00
	cmp 	r0,r2
	orrne 	r1,r1,#BAD_Rd
	mov 	r2,#8
	orr 	r1,r1,#0x80000000
	ldr 	r0,=szLDR
	add 	r8,r8,#8

	@ ]-#
	mov 	r1,#0
	ldr 	r0,=romvar
	mov 	r2,r0
	sub 	r3,r0,#0xff
	ldr 	r0,[r0],#-0xff
	cmp 	r0,#0x80
	orrne 	r1,r1,#BAD_Rd
	ldr 	r0,[r2],#-0xff
	cmp 	r0,#0x80
	orrne 	r1,r1,#BAD_Rd
	cmp 	r2,r3
	orrne 	r1,r1,#BAD_Rn
	@ Test non word-aligned load
	ldr 	r0,=romvar2+2
	ldr 	r0,[r0],#-5
	ldr 	r2,=0x8f00ff00
	cmp 	r0,r2
	orrne 	r1,r1,#BAD_Rd
	mov 	r2,#9
	orr 	r1,r1,#0x80000000
	ldr 	r0,=szLDR
	add 	r8,r8,#8


	@ ]+R
	mov 	r1,#0
	ldr 	r0,=romvar
	mov 	r2,r0
	add 	r5,r0,#8
	mov 	r3,r0
	mov 	r4,#2
	ldr 	r0,[r0],r4, lsr #1
	cmp 	r0,#0x80
	orrne 	r1,r1,#BAD_Rd
	ldr 	r0,[r2],r4, lsl #2
	cmp 	r0,#0x80
	orrne 	r1,r1,#BAD_Rd
	cmp 	r2,r5
	orrne 	r1,r1,#BAD_Rn

	ldr 	r2,=romvar
	mov 	r0,#123
	add 	r3,r2,r0
	ldr 	r0,[r2],r0
	cmp 	r0,#0x80
	orrne 	r1,r1,#BAD_Rd
	cmp 	r2,r3
	orrne 	r1,r1,#BAD_Rn
	
	ldr 	r2,=romvar
	sub 	r4,r2,#1
	mov 	r3,#0xC0000000
	ldr 	r0,[r2],r3, asr #32
	cmp 	r0,#0x80
	orrne 	r1,r1,#BAD_Rd
	cmp 	r2,r4
	orrne 	r1,r1,#BAD_Rn

	ldr 	r2,=romvar
	sub 	r4,r2,#2
	ldr 	r3,=0xfffffffc
	adds 	r5,r3,r3		@ set carry
	ldr 	r0,[r2],r3, rrx
	orrcc 	r1,r1,#1
	cmp 	r0,#0x80
	orrne 	r1,r1,#BAD_Rd
	cmp 	r2,r4
	orrne 	r1,r1,#BAD_Rn

	@ Test non word-aligned load
	ldr 	r0,=romvar2+2
	mov 	r2,#1
	ldr 	r0,[r0],r2
	ldr 	r2,=0x8f00ff00
	cmp 	r0,r2
	orrne 	r1,r1,#BAD_Rd
	
	mov 	r2,#10
	orr 	r1,r1,#0x80000000
	ldr 	r0,=szLDR
	bl 	DrawResult
	add 	r8,r8,#8


	@ ]-R
	mov 	r1,#0
	ldr 	r0,=romvar
	mov 	r2,r0
	sub 	r5,r0,#16
	mov 	r3,r0
	mov 	r4,#2
	ldr 	r0,[r0],-r4, lsr #1
	cmp 	r0,#0x80
	orrne 	r1,r1,#BAD_Rd
	ldr 	r0,[r2],-r4, lsl #3
	cmp 	r0,#0x80
	orrne 	r1,r1,#BAD_Rd
	cmp 	r2,r5
	orrne 	r1,r1,#BAD_Rn

	ldr	r2,=romvar
	mov 	r0,#123
	sub 	r3,r2,r0
	ldr 	r0,[r2],-r0
	cmp 	r0,#0x80
	orrne 	r1,r1,#BAD_Rd
	cmp 	r2,r3
	orrne 	r1,r1,#BAD_Rn
	
	ldr 	r2,=romvar
	add 	r4,r2,#1
	mov 	r3,#0xC0000000
	ldr 	r0,[r2],-r3, asr #32
	cmp 	r0,#0x80
	orrne 	r1,r1,#BAD_Rd
	cmp 	r2,r4
	orrne 	r1,r1,#BAD_Rn

	ldr 	r2,=romvar
	add 	r4,r2,#2
	ldr 	r3,=0xfffffffc
	adds 	r5,r3,r3		@ set carry
	ldr 	r0,[r2],-r3, rrx
	orrcc 	r1,r1,#1
	cmp 	r0,#0x80
	orrne 	r1,r1,#BAD_Rd
	cmp 	r2,r4
	orrne 	r1,r1,#BAD_Rn

	@ Test non word-aligned load
	ldr 	r0,=romvar2+2
	mov 	r2,#5
	ldr 	r0,[r0],-r2
	ldr 	r2,=0x8f00ff00
	cmp 	r0,r2
	orrne 	r1,r1,#BAD_Rd
	
	mov 	r2,#11
	orr 	r1,r1,#0x80000000
	ldr 	r0,=szLDR
	
	swi    #0x900001 


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
