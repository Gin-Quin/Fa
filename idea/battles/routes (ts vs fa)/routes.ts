let routes : Route[] = [
	{
		key: 'deal_with_fds',  // used for translation
		path: '/',
		pattern: /^\/((consult-fds|my-fds|my-safebox|my-chemical-base|my-tools|shopping-cart)(\/.*)?)$/
	},
	{
		key: 'expert_tools',
		path: '/solutions',
		sub: {
			info_send_proof: {
				path: '/info-send-proof',
				text: "Preuves d'envoi",
			},
			info_push_and_proof : {
				path: '/info-push-and-proof',
				text: 'Module PUSH & PROOF',
			},
			info_quifds_witness : {
				path: '/info-quickfds-witness',
				text: "Quick-FDS WITNESS",
			},
			info_safebox : {
				path: '/info-safebox',
				text: "Programme SafeBox",
			},
			info_hosting : {
				path: '/info-hosting',
				text: "Hébergement à valeur ajoutée",
			},
		}
	},
	{
		key: 'solution_turnkey',
		path: '/services',
		sub: {
			services_distrib: {
				path: '/services-distrib',
				text: "navbar_title_distrib_FDS_main",
				sub: {
					conform: {
						path: '/services-distrib#',
						text: "Conformité réglementaire",
					},
					externalisation: {
						path: '/services-distrib#externalisation',
						text: "Externalisation des envois de FDS : Push & Proof",
					},
					witness: {
						path: '/services-distrib#witness',
						text: "Certification des envois de FDS : Quick-FDS Witness",
					},
					services: {
						path: '/services-distrib#services',
						text: "Services pour vos clients",
					},
				}
			},
			services_manage: {
				path: '/services-manage',
				text: "navbar_title_manage_FDS_main",
				sub: {
					intro: {
						path: '/services-manage#',
						text: "Introduction",
					},
					collecte: {
						path: '/services-manage#collecte',
						text: "Collecte des FDS",
					},
					verif: {
						path: '/services-manage#verif',
						text: "Vérification des FDS",
					},
					extract: {
						path: '/services-manage#extract',
						text: "Extraction des données",
					},
					analyses: {
						path: '/services-manage#analyses',
						text: "Analyses réglementaires",
					},
					diffusion: {
						path: '/services-manage#diffusion',
						text: "Diffusion au personnel  ",
					},
					archivage: {
						path: '/services-manage#archivage',
						text: "Archivages des FDS",
					},

				}
			},
		}
	},
	{
		key: 'consulting_and_formation',
		path: '/consulting'
	},
	{
		key: 'news',
		path: '/news'
	},
	{
		key: 'who_we_are',
		path: '/about-us'
	},
]
