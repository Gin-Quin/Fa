let routes: Route[] = [
	{
		key: "deal_with_fds", // used for translation
		path: "/",
		pattern:
			/^\/((consult-fds|my-fds|my-safebox|my-chemical-base|my-tools|shopping-cart)(\/.*)?)$/,
	},
	{
		key: "expert_tools",
		path: "/solutions",
		sub: {
			info_send_proof: {
				path: "/info-send-proof",
				text: "Preuves d'envoi",
			},
			info_push_and_proof: {
				path: "/info-push-and-proof",
				text: "Module PUSH & PROOF",
			},
			info_quifds_witness: {
				path: "/info-quickfds-witness",
				text: "Quick-FDS WITNESS",
			},
			info_safebox: {
				path: "/info-safebox",
				text: "Programme SafeBox",
			},
			info_hosting: {
				path: "/info-hosting",
				text: "Hébergement à valeur ajoutée",
			},
		},
	},
	{
		key: "consulting_and_formation",
		path: "/consulting",
	},
	{
		key: "news",
		path: "/news",
	},
	{
		key: "who_we_are",
		path: "/about-us",
	},
]
