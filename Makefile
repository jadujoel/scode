
main:
	cargo build --release

sounds:
	cargo run --release -- --indir="../sounds/src/packages" --outdir="encoded"

clean:
	rm -rf out .cache encoded
