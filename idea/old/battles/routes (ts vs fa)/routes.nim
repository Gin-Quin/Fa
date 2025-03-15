let routes = @[
  Route(
    key: "deal_with_fds",
    path: "/",
    pattern:
      re"^/((consult-fds|my-fds|my-safebox|my-chemical-base|my-tools|shopping-cart)(/.*)?)\$"
  ),
  Route(
    key: "expert_tools",
    path: "/solutions",
    sub: {
      "info_send_proof": SubRoute(
        path: "/info-send-proof",
        text: "Preuves d'envoi"
      ),
      "info_push_and_proof": SubRoute(
        path: "/info-push-and-proof",
        text: "Module PUSH & PROOF"
      ),
      "info_quifds_witness": SubRoute(
        path: "/info-quickfds-witness",
        text: "Quick-FDS WITNESS"
      ),
      "info_safebox": SubRoute(
        path: "/info-safebox",
        text: "Programme SafeBox"
      ),
      "info_hosting": SubRoute(
        path: "/info-hosting",
        text: "Hébergement à valeur ajoutée"
      )
    }.toTable
  ),
  Route(
    key: "consulting_and_formation",
    path: "/consulting"
  ),
  Route(
    key: "news",
    path: "/news"
  ),
  Route(
    key: "who_we_are",
    path: "/about-us"
  )
]
