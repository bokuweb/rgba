BIOS Functions
The BIOS includes several System Call Functions which can be accessed by SWI instructions. Incoming parameters are usually passed through registers R0,R1,R2,R3. Outgoing registers R0,R1,R3 are typically containing either garbage, or return value(s). All other registers (R2,R4-R14) are kept unchanged.

Caution
When invoking SWIs from inside of ARM state specify SWI NN*10000h, instead of SWI NN as in THUMB state.

Overview
BIOS Function Summary
BIOS Differences between GBA and NDS functions
All Functions Described
BIOS Arithmetic Functions
BIOS Rotation/Scaling Functions
BIOS Decompression Functions
BIOS Memory Copy
BIOS Halt Functions
BIOS Reset Functions
BIOS Misc Functions
BIOS Multi Boot (Single Game Pak)
BIOS Sound Functions
BIOS SHA1 Functions (DSi only)
BIOS RSA Functions (DSi only)
RAM Usage, BIOS Dumps
BIOS RAM Usage
BIOS Dumping
How BIOS Processes SWIs
SWIs can be called from both within THUMB and ARM mode. In ARM mode, only the upper 8bit of the 24bit comment field are interpreted.

Each time when calling a BIOS function 4 words (SPSR, R11, R12, R14) are saved on Supervisor stack (_svc). Once it has saved that data, the SWI handler switches into System mode, so that all further stack operations are using user stack.

In some cases the BIOS may allow interrupts to be executed from inside of the SWI procedure. If so, and if the interrupt handler calls further SWIs, then care should be taken that the Supervisor Stack does not overflow.

BIOS Function Summary
  GBA  NDS7 NDS9 DSi7 DSi9 Basic Functions
  00h  00h  00h  -    -    SoftReset
  01h  -    -    -    -    RegisterRamReset
  02h  06h  06h  06h  06h  Halt
  03h  07h  -    07h  -    Stop/Sleep
  04h  04h  04h  04h  04h  IntrWait       ;DSi7/DSi9: both bugged?
  05h  05h  05h  05h  05h  VBlankIntrWait ;DSi7/DSi9: both bugged?
  06h  09h  09h  09h  09h  Div
  07h  -    -    -    -    DivArm
  08h  0Dh  0Dh  0Dh  0Dh  Sqrt
  09h  -    -    -    -    ArcTan
  0Ah  -    -    -    -    ArcTan2
  0Bh  0Bh  0Bh  0Bh  0Bh  CpuSet
  0Ch  0Ch  0Ch  0Ch  0Ch  CpuFastSet
  0Dh  -    -    -    -    GetBiosChecksum
  0Eh  -    -    -    -    BgAffineSet
  0Fh  -    -    -    -    ObjAffineSet
  GBA  NDS7 NDS9 DSi7 DSi9 Decompression Functions
  10h  10h  10h  10h  10h  BitUnPack
  11h  11h  11h  11h  11h  LZ77UnCompReadNormalWrite8bit   ;"Wram"
  12h  -    -    -    -    LZ77UnCompReadNormalWrite16bit  ;"Vram"
  -    -    -    01h  01h  LZ77UnCompReadByCallbackWrite8bit
  -    12h  12h  02h  02h  LZ77UnCompReadByCallbackWrite16bit
  -    -    -    19h  19h  LZ77UnCompReadByCallbackWrite16bit (same as above)
  13h  -    -    -    -    HuffUnCompReadNormal
  -    13h  13h  13h  13h  HuffUnCompReadByCallback
  14h  14h  14h  14h  14h  RLUnCompReadNormalWrite8bit     ;"Wram"
  15h  -    -    -    -    RLUnCompReadNormalWrite16bit    ;"Vram"
  -    15h  15h  15h  15h  RLUnCompReadByCallbackWrite16bit
  16h  -    16h  -    16h  Diff8bitUnFilterWrite8bit       ;"Wram"
  17h  -    -    -    -    Diff8bitUnFilterWrite16bit      ;"Vram"
  18h  -    18h  -    18h  Diff16bitUnFilter
  GBA  NDS7 NDS9 DSi7 DSi9 Sound (and Multiboot/HardReset/CustomHalt)
  19h  08h  -    08h  -    SoundBias
  1Ah  -    -    -    -    SoundDriverInit
  1Bh  -    -    -    -    SoundDriverMode
  1Ch  -    -    -    -    SoundDriverMain
  1Dh  -    -    -    -    SoundDriverVSync
  1Eh  -    -    -    -    SoundChannelClear
  1Fh  -    -    -    -    MidiKey2Freq
  20h  -    -    -    -    SoundWhatever0
  21h  -    -    -    -    SoundWhatever1
  22h  -    -    -    -    SoundWhatever2
  23h  -    -    -    -    SoundWhatever3
  24h  -    -    -    -    SoundWhatever4
  25h  -    -    -    -    MultiBoot
  26h  -    -    -    -    HardReset
  27h  1Fh  -    1Fh  -    CustomHalt
  28h  -    -    -    -    SoundDriverVSyncOff
  29h  -    -    -    -    SoundDriverVSyncOn
  2Ah  -    -    -    -    SoundGetJumpList
  GBA  NDS7 NDS9 DSi7 DSi9 New NDS Functions
  -    03h  03h  03h  03h  WaitByLoop
  -    0Eh  0Eh  0Eh  0Eh  GetCRC16
  -    0Fh  0Fh  -    -    IsDebugger
  -    1Ah  -    1Ah  -    GetSineTable
  -    1Bh  -    1Bh  -    GetPitchTable (DSi7: bugged)
  -    1Ch  -    1Ch  -    GetVolumeTable
  -    1Dh  -    1Dh  -    GetBootProcs (DSi7: only 1 proc)
  -    -    1Fh  -    1Fh  CustomPost
  GBA  NDS7 NDS9 DSi7 DSi9 New DSi Functions (RSA/SHA1)
  -    -    -    20h  20h  RSA_Init_crypto_heap
  -    -    -    21h  21h  RSA_Decrypt
  -    -    -    22h  22h  RSA_Decrypt_Unpad
  -    -    -    23h  23h  RSA_Decrypt_Unpad_OpenPGP_SHA1
  -    -    -    24h  24h  SHA1_Init
  -    -    -    25h  25h  SHA1_Update
  -    -    -    26h  26h  SHA1_Finish
  -    -    -    27h  27h  SHA1_Init_update_fin
  -    -    -    28h  28h  SHA1_Compare_20_bytes
  -    -    -    29h  29h  SHA1_Random_maybe
  GBA  NDS7 NDS9 DSi7 DSi9 Invalid Functions
  2Bh+ 20h+ 20h+ -    -    Crash (SWI xxh..FFh do jump to garbage addresses)
  -    xxh  xxh  -    -    Jump to 0   (on any SWI numbers not listed above)
  -    -    -    12h  12h  No function (ignored)
  -    -    -    2Bh  2Bh  No function (ignored)
  -    -    -    40h+ 40h+ Mirror      (SWI 40h..FFh mirror to 00h..3Fh)
  -    -    -    xxh  xxh  Hang        (on any SWI numbers not listed above)
Invalid NDS functions: NDS7 SWI 01h, 02h, 0Ah, 16h-19h, 1Eh, and NDS9 SWI 01h, 02h, 07h, 08h, 0Ah, 17h, 19h-1Eh will jump to zero (ie. to the NDS7 reset vector, or to NDS9 unused (usually PU-locked ITCM) memory, which will be both redirected to the debug handler, if any).

Invalid DSi functions: DSi9 SWI 00h, 07h-08h, 0Ah, 0Fh, 17h, 1Ah-1Eh, 2Ah, 2Ch-3Fh do hang in endless loop.

BIOS Differences between GBA and NDS functions
Differences between GBA and NDS BIOS functions
SoftReset uses different addresses

SWI numbers for Halt, Stop/Sleep, Div, Sqrt have changed

Halt destroys r0 on NDS9, IntrWait bugged on NDS9

CpuFastSet allows 4-byte blocks (nice), but…

CpuFastSet works very SLOW because of a programming bug (uncool)

Some of the decompression functions are now using callbacks

SoundBias uses new delay parameter

And, a number of GBA functions have been removed, and some new NDS functions have been added, see:

BIOS Function Summary
BIOS Arithmetic Functions
Div

DivArm

Sqrt

ArcTan

ArcTan2

SWI 06h (GBA) or SWI 09h (NDS7/NDS9/DSi7/DSi9) - Div
Signed Division, r0/r1.

  r0  signed 32bit Number
  r1  signed 32bit Denom
Return:

  r0  Number DIV Denom ;signed
  r1  Number MOD Denom ;signed
  r3  ABS (Number DIV Denom) ;unsigned
For example, incoming -1234, 10 should return -123, -4, +123.

The function usually gets caught in an endless loop upon division by zero.

Note: The NDS9 and DSi9 additionally support hardware division, by math coprocessor, accessed via I/O Ports, however, the SWI function is a raw software division.

SWI 07h (GBA) - DivArm
Same as above (SWI 06h Div), but incoming parameters are exchanged, r1/r0 (r0=Denom, r1=number). For compatibility with ARM’s library. Slightly slower (3 clock cycles) than SWI 06h.

SWI 08h (GBA) or SWI 0Dh (NDS7/NDS9/DSi7/DSi9) - Sqrt
Calculate square root.

  r0   unsigned 32bit number
Return:

  r0   unsigned 16bit number
The result is an integer value, so Sqrt(2) would return 1, to avoid this inaccuracy, shift left incoming number by 2*N as much as possible (the result is then shifted left by 1*N). Ie. Sqrt(2 shl 30) would return 1.41421 shl 15.

Note: The NDS9 and DSi9 additionally support hardware square root calculation, by math coprocessor, accessed via I/O Ports, however, the SWI function is a raw software calculation.

SWI 09h (GBA) - ArcTan
Calculates the arc tangent.

  r0   Tan, 16bit (1bit sign, 1bit integral part, 14bit decimal part)
Return:

  r0   "-PI/2<THETA/<PI/2" in a range of C000h-4000h.
Note: there is a problem in accuracy with “THETA<-PI/4, PI/4<THETA”.

SWI 0Ah (GBA) - ArcTan2
Calculates the arc tangent after correction processing.

Use this in normal situations.

  r0   X, 16bit (1bit sign, 1bit integral part, 14bit decimal part)
  r1   Y, 16bit (1bit sign, 1bit integral part, 14bit decimal part)
Return:

  r0   0000h-FFFFh for 0<=THETA<2PI.
BIOS Rotation/Scaling Functions
BgAffineSet

ObjAffineSet

SWI 0Eh (GBA) - BgAffineSet
Used to calculate BG Rotation/Scaling parameters.

  r0   Pointer to Source Data Field with entries as follows:
        s32  Original data's center X coordinate (8bit fractional portion)
        s32  Original data's center Y coordinate (8bit fractional portion)
        s16  Display's center X coordinate
        s16  Display's center Y coordinate
        s16  Scaling ratio in X direction (8bit fractional portion)
        s16  Scaling ratio in Y direction (8bit fractional portion)
        u16  Angle of rotation (8bit fractional portion) Effective Range 0-FFFF
  r1   Pointer to Destination Data Field with entries as follows:
        s16  Difference in X coordinate along same line
        s16  Difference in X coordinate along next line
        s16  Difference in Y coordinate along same line
        s16  Difference in Y coordinate along next line
        s32  Start X coordinate
        s32  Start Y coordinate
  r2   Number of Calculations
Return: No return value, Data written to destination address.

SWI 0Fh (GBA) - ObjAffineSet
Calculates and sets the OBJ’s affine parameters from the scaling ratio and angle of rotation.

The affine parameters are calculated from the parameters set in Srcp.

The four affine parameters are set every Offset bytes, starting from the Destp address.

If the Offset value is 2, the parameters are stored contiguously. If the value is 8, they match the structure of OAM.

When Srcp is arrayed, the calculation can be performed continuously by specifying Num.

  r0   Source Address, pointing to data structure as such:
        s16  Scaling ratio in X direction (8bit fractional portion)
        s16  Scaling ratio in Y direction (8bit fractional portion)
        u16  Angle of rotation (8bit fractional portion) Effective Range 0-FFFF
  r1   Destination Address, pointing to data structure as such:
        s16  Difference in X coordinate along same line
        s16  Difference in X coordinate along next line
        s16  Difference in Y coordinate along same line
        s16  Difference in Y coordinate along next line
  r2   Number of calculations
  r3   Offset in bytes for parameter addresses (2=continuous, 8=OAM)
Return: No return value, Data written to destination address.

For both Bg- and ObjAffineSet, Rotation angles are specified as 0-FFFFh (covering a range of 360 degrees), however, the GBA BIOS recurses only the upper 8bit; the lower 8bit may contain a fractional portion, but it is ignored by the BIOS.

BIOS Decompression Functions
BitUnPack

Diff8bitUnFilter

HuffUnComp

LZ77UnComp

RLUnComp

Decompression Read/Write Variants
  ReadNormal:      Fast (src must be memory mapped)
  ReadByCallback:  Slow (src can be non-memory, eg. serial Firmware SPI bus)
  Write8bitUnits:  Fast (dest must support 8bit writes, eg. not VRAM)
  Write16bitUnits: Slow (dest must be halfword-aligned) (for VRAM)
BitUnPack - SWI 10h (GBA/NDS7/NDS9/DSi7/DSi9)
Used to increase the color depth of bitmaps or tile data. For example, to convert a 1bit monochrome font into 4bit or 8bit GBA tiles. The Unpack Info is specified separately, allowing to convert the same source data into different formats.

  r0  Source Address      (no alignment required)
  r1  Destination Address (must be 32bit-word aligned)
  r2  Pointer to UnPack information:
       16bit  Length of Source Data in bytes     (0-FFFFh)
       8bit   Width of Source Units in bits      (only 1,2,4,8 supported)
       8bit   Width of Destination Units in bits (only 1,2,4,8,16,32 supported)
       32bit  Data Offset (Bit 0-30), and Zero Data Flag (Bit 31)
      The Data Offset is always added to all non-zero source units.
      If the Zero Data Flag was set, it is also added to zero units.
Data is written in 32bit units, Destination can be Wram or Vram. The size of unpacked data must be a multiple of 4 bytes. The width of source units (plus the offset) should not exceed the destination width.

Return: No return value, Data written to destination address.

Diff8bitUnFilterWrite8bit (Wram) - SWI 16h (GBA/NDS9/DSi9)
Diff8bitUnFilterWrite16bit (Vram) - SWI 17h (GBA)
Diff16bitUnFilter - SWI 18h (GBA/NDS9/DSi9)
These aren’t actually real decompression functions, destination data will have exactly the same size as source data. However, assume a bitmap or wave form to contain a stream of increasing numbers such like 10..19, the filtered/unfiltered data would be:

  unfiltered:   10  11  12  13  14  15  16  17  18  19
  filtered:     10  +1  +1  +1  +1  +1  +1  +1  +1  +1
In this case using filtered data (combined with actual compression algorithms) will obviously produce better compression results.

Data units may be either 8bit or 16bit used with Diff8bit or Diff16bit functions respectively.

  r0  Source address (must be aligned by 4) pointing to data as follows:
       Data Header (32bit)
         Bit 0-3   Data size (must be 1 for Diff8bit, 2 for Diff16bit)
         Bit 4-7   Type (must be 8 for DiffFiltered)
         Bit 8-31  24bit size after decompression
       Data Units (each 8bit or 16bit depending on used SWI function)
         Data0          ;original data
         Data1-Data0    ;difference data
         Data2-Data1    ;...
         Data3-Data2
         ...
  r1  Destination address
Return: No return value, Data written to destination address.

HuffUnCompReadNormal - SWI 13h (GBA)
HuffUnCompReadByCallback - SWI 13h (NDS/DSi)
The decoder starts in root node, the separate bits in the bitstream specify if the next node is node0 or node1, if that node is a data node, then the data is stored in memory, and the decoder is reset to the root node. The most often used data should be as close to the root node as possible. For example, the 4-byte string “Huff” could be compressed to 6 bits: 10-11-0-0, with root.0 pointing directly to data “f”, and root.1 pointing to a child node, whose nodes point to data “H” and data “u”.

Data is written in units of 32bits, if the size of the compressed data is not a multiple of 4, please adjust it as much as possible by padding with 0.

Align the source address to a 4Byte boundary.

  r0  Source Address, aligned by 4, pointing to:
       Data Header (32bit)
         Bit0-3   Data size in bit units (normally 4 or 8)
         Bit4-7   Compressed type (must be 2 for Huffman)
         Bit8-31  24bit size of decompressed data in bytes
       Tree Size (8bit)
         Bit0-7   Size of Tree Table/2-1 (ie. Offset to Compressed Bitstream)
       Tree Table (list of 8bit nodes, starting with the root node)
        Root Node and Non-Data-Child Nodes are:
         Bit0-5   Offset to next child node,
                  Next child node0 is at (CurrentAddr AND NOT 1)+Offset*2+2
                  Next child node1 is at (CurrentAddr AND NOT 1)+Offset*2+2+1
         Bit6     Node1 End Flag (1=Next child node is data)
         Bit7     Node0 End Flag (1=Next child node is data)
        Data nodes are (when End Flag was set in parent node):
         Bit0-7   Data (upper bits should be zero if Data Size is less than 8)
       Compressed Bitstream (stored in units of 32bits)
         Bit0-31  Node Bits (Bit31=First Bit)  (0=Node0, 1=Node1)
  r1  Destination Address
  r2  Callback temp buffer      ;\for NDS/DSi "ReadByCallback" variants only
  r3  Callback structure        ;/(see Callback notes below)
Return: No return value, Data written to destination address.

LZ77UnCompReadNormalWrite8bit (Wram) - SWI 11h (GBA/NDS7/NDS9/DSi7/DSi9)
LZ77UnCompReadNormalWrite16bit (Vram) - SWI 12h (GBA)
LZ77UnCompReadByCallbackWrite8bit - SWI 01h (DSi7/DSi9)
LZ77UnCompReadByCallbackWrite16bit - SWI 12h (NDS), SWI 02h or 19h (DSi)
Expands LZ77-compressed data. The Wram function is faster, and writes in units of 8bits. For the Vram function the destination must be halfword aligned, data is written in units of 16bits.

CAUTION: Writing 16bit units to [dest-1] instead of 8bit units to [dest] means that reading from [dest-1] won’t work, ie. the “Vram” function works only with disp=001h..FFFh, but not with disp=000h.

If the size of the compressed data is not a multiple of 4, please adjust it as much as possible by padding with 0. Align the source address to a 4-Byte boundary.

  r0  Source address, pointing to data as such:
       Data header (32bit)
         Bit 0-3   Reserved
         Bit 4-7   Compressed type (must be 1 for LZ77)
         Bit 8-31  Size of decompressed data
       Repeat below. Each Flag Byte followed by eight Blocks.
       Flag data (8bit)
         Bit 0-7   Type Flags for next 8 Blocks, MSB first
       Block Type 0 - Uncompressed - Copy 1 Byte from Source to Dest
         Bit 0-7   One data byte to be copied to dest
       Block Type 1 - Compressed - Copy N+3 Bytes from Dest-Disp-1 to Dest
         Bit 0-3   Disp MSBs
         Bit 4-7   Number of bytes to copy (minus 3)
         Bit 8-15  Disp LSBs
  r1  Destination address
  r2  Callback parameter        ;\for NDS/DSi "ReadByCallback" variants only
  r3  Callback structure        ;/(see Callback notes below)
Return: No return value.

RLUnCompReadNormalWrite8bit (Wram) - SWI 14h (GBA/NDS7/NDS9/DSi7/DSi9)
RLUnCompReadNormalWrite16bit (Vram) - SWI 15h (GBA)
RLUnCompReadByCallbackWrite16bit - SWI 15h (NDS7/NDS9/DSi7/DSi9)
Expands run-length compressed data. The Wram function is faster, and writes in units of 8bits. For the Vram function the destination must be halfword aligned, data is written in units of 16bits.

If the size of the compressed data is not a multiple of 4, please adjust it as much as possible by padding with 0. Align the source address to a 4Byte boundary.

  r0  Source Address, pointing to data as such:
       Data header (32bit)
         Bit 0-3   Reserved
         Bit 4-7   Compressed type (must be 3 for run-length)
         Bit 8-31  Size of decompressed data
       Repeat below. Each Flag Byte followed by one or more Data Bytes.
       Flag data (8bit)
         Bit 0-6   Expanded Data Length (uncompressed N-1, compressed N-3)
         Bit 7     Flag (0=uncompressed, 1=compressed)
       Data Byte(s) - N uncompressed bytes, or 1 byte repeated N times
  r1  Destination Address
  r2  Callback parameter        ;\for NDS/DSi "ReadByCallback" variants only
  r3  Callback structure        ;/(see Callback notes below)
Return: No return value, Data written to destination address.