.PHONY: run
run:
	cargo run --release

.PHONY: deploy
deploy:
	rsync -avh --delete ./web/{index.html,index.txt,style.css} web:/var/www/reverie.ysap.sh/
