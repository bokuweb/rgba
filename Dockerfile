  
FROM rustlang/rust:nightly

WORKDIR /code

# Install node
RUN wget http://ftp.gnu.org/gnu/binutils/binutils-2.27.tar.gz && \
  tar -zxvf binutils-2.27.tar.gz && \
  cd binutils-2.27 && \
  ./configure --target=arm-none-eabi && \
  make && \
  make install

RUN cargo install cargo-xbuild
RUN rustup component add rust-src

CMD ["make"]