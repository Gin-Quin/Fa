# 3897 caractères (-1167, -23% !)
@author Lepzulnag
@description
	"That's awesome, right ?"

extends Vue

import Route 'from' types
import Component, Watch 'from' 'vue-property-decorator'
use /components/LanguageSwitcher
use /components/popins/PopinLogin
use /components/SplashScreen


showLoginPopup = false
showRegisterPopup = false
isNavForced = false
displayedMenu = ''

tabToDropDown = ''

keyRouteActif = ''
routes: [Route]

isLoggedIn -> $store.getters['account/isLoggedIn']
displayName -> $store.getters['account/profil'].first_name + ' ' + $store.getters['account/profil'].last_name.toUpperCase()
isHeaderMinimal -> isLoggedIn && !isNavForced


hardWatch $route
	tabToDropDown = ''
	keyRouteActif = ''

	for route in routes
		if route.sub
			for subKey in route.sub
				if isSubActif route.sub[subKey].path
					keyRouteActif = subKey
		else if isActive route
			keyRouteActif = route.key

toast text:String
	$bvToast.toast:
		text
		options:
			autoHideDelay = 3000
			appendToast = false

moveMiniMenu path:String
	for key in routes
		routes[key].collapsed = false
	@ts_ignore
   $refs.miniMenuButton.click
   $router.push path

moveMenu path:String
	for route in routes
		route.collapsed = false
	openRoute '' 
	$router.push path, coco
	hideMenu

forceNav
	isNavForced = !isNavForced

logout
	$router.push "/account/logout"

goProfil
	$router.push "/account/profil"


displayFullMenu routeKey:String
	tabToDropDown = if tabToDropDown is routeKey then '' else routeKey 
	if routeKey == routes[0].key
		$router.push routes[0].path

hideFullMenu
	openRoute ''

showMenu key:String
	print "SHOW MENU", key
	displayedMenu = key

hideMenu
	print "HIDE MENU"
	displayedMenu = ''


openPopup name:String
	when name
	is 'login'
		showLoginPopup = true
	is 'register'
		showRegisterPopup = true


# the different navigation routes
routes: [Route]
	-	key = 'deal_with_fds' # used for translation
		path = '/'
		pattern = /^/((consult-fds|my-fds|my-safebox|my-chemical-base|my-tools|shopping-cart)(.*)?)$/
	
	-	key = 'expert_tools'
		path = '/solutions'
		sub:
			info_send_proof:
				path = '/info-send-proof'
				text = "Preuves d'envoi"
			info_push_and_proof:
				path = '/info-push-and-proof'
				text = 'Module PUSH & PROOF'
			info_quifds_witness:
				path = '/info-quickfds-witness'
				text = "Quick-FDS WITNESS"
			info_safebox: 
				path = '/info-safebox'
				text = "Programme SafeBox"
			info_hosting: 
				path = '/info-hosting'
				text = "Hébergement à valeur ajoutée"
	
	-	key = 'solution_turnkey'
		path = '/services'
		sub:
			services_distrib: 
				path = '/services-distrib'
				text = "navbar_title_distrib_FDS_main"
				sub: 
					conform: 
						- '/services-distrib#'
						- "Conformité réglementaire"
					
					externalisation: 
						- '/services-distrib#externalisation'
						- "Externalisation des envois de FDS : Push & Proof"
					
					witness: 
						path = '/services-distrib#witness'
						text = "Certification des envois de FDS : Quick-FDS Witness"
					
					services: 
						path = '/services-distrib#services'
						text = "Services pour vos clients"
					
			services_manage: 
				path = '/services-manage'
				text = "navbar_title_manage_FDS_main"
				sub: 
					intro: 
						path = '/services-manage#'
						text = "Introduction"
					
					collecte: 
						path = '/services-manage#collecte'
						text = "Collecte des FDS"
					
					verif: 
						path = '/services-manage#verif'
						text = "Vérification des FDS"
					
					extract: 
						path = '/services-manage#extract'
						text = "Extraction des données"
					
					analyses: 
						path = '/services-manage#analyses'
						text = "Analyses réglementaires"
					
					diffusion: 
						path = '/services-manage#diffusion'
						text = "Diffusion au personnel  "
					
					archivage: 
						path = '/services-manage#archivage'
						text = "Archivages des FDS"

	-	key = 'consulting_and_formation'
		path = '/consulting'
	
	-	key = 'news'
		path = '/news'
	
	-	key = 'who_we_are'
		path = '/about-us'