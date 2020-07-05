int main()
{
    *(unsigned int*)0x04000000=0x0403;
    ((unsigned short*)0x06000000)[120+80*240]=0x001f;
    while(1);
}