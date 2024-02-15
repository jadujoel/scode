
main:
	cargo run

prod:
	cargo run --release

sounds:
	cargo run --release -- --indir="../sounds/src/packages" --outdir="encoded"

build:
	cargo build --release

run:
	./target/release/ecas-encoder

clean:
	rm -rf out .cache encoded
