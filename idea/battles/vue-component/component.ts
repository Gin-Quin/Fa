// 4901 caractères (+1011, +25% characters)
import {Route} from './types'
import LanguageSwitcher from '@/components/LanguageSwitcher/component.vue'
import PopinLogin from '@/components/popins/PopinLogin/component.vue'
import SplashScreen from '@/components/SplashScreen/component.vue';

import {Vue, Component, Watch} from 'vue-property-decorator'


export default class SiteNavigation extends Vue {
	showLoginPopup:boolean = false;
	showRegisterPopup:boolean = false;
	isNavForced = false;
	displayedMenu = ''

	tabToDropDown = '';

	keyRouteActif = '';

	onChangeRoute() {
		this.tabToDropDown = '';
		this.keyRouteActif = '';

		for(const route of routes) {
			if(route.sub) {
				for(const subKey in route.sub) {
					if(this.isSubActif(route.sub[subKey].path)) {
						this.keyRouteActif = subKey;
					}
				}
			} else {
				if(this.isActive(route)) {
					this.keyRouteActif = route.key;
				}
			}
		}
	}

	toast(text:string) {
		this.$bvToast.toast(text, {
			autoHideDelay: 3000,
			appendToast: false
		});
	}

	get isLoggedIn() {
		return this.$store.getters['account/isLoggedIn']
	}
	get displayName() {
		return this.$store.getters['account/profil'].first_name + ' ' + this.$store.getters['account/profil'].last_name.toUpperCase();
	}
	get isHeaderMinimal() {
		return this.isLoggedIn && !this.isNavForced
	}

	moveMiniMenu(path:string) {
		for(const key in this.routes) {
			this.routes[key].collapsed = false;
		}
	   this.$refs.miniMenuButton.click();
	   this.$router.push(path);
	}

	moveMenu(path:string) {

		for(const routeKey in this.routes) {
			this.routes[routeKey].collapsed = false;
		}

		this.openRoute('');
		this.$router.push(path);
		this.hideMenu()
	}

	forceNav() {
		this.isNavForced = !this.isNavForced;
	}

	logout() {
		this.$router.push("/account/logout");
	}
	goProfil() {
		this.$router.push("/account/profil");
	}

	displayFullMenu(routeKey:string) {
		this.tabToDropDown = this.tabToDropDown === routeKey ? '' : routeKey;
		if(routeKey === routes[0].key) {
			this.$router.push(routes[0].path);
		}
	}

	hideFullMenu() {
		this.openRoute('');
	}

	showMenu(key:string) {
		console.log("SHOW MENU", key)
		this.displayedMenu = key
	}

	hideMenu() {
		console.log("HIDE MENU")
		this.displayedMenu = ''
	}


	openPopup(name:string) {
		switch(name) {
			case 'login':
			this.showLoginPopup = true;
			break;
			case 'register':
			this.showRegisterPopup = true;
			break;
			default:
			break;
		}
	}

	// the different navigation routes
	routes : Route[] = [
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
}