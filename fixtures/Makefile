
build:
	docker run -v `pwd`/hello:/hello -w /hello shumon84/gba make
	cd dot_rs && docker run --rm -v `pwd`:/code bokuweb/rust-gba