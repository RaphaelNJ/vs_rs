# vs_rs

## Description
Ce projet est une alternative à Scratch qui offre une interface inspirée du blueprint d'Unreal Engine. Il a été entrepris car je trouve que Scratch utilise une approche peu ergonomique. De plus Scratch est interprété alors que ce projet converti le transforme en [code fennel](https://fennel-lang.org/) avant de l'executer, offrant une bien meilleure vitesse. L'objectif était donc de créer un langage de programmation visuel personnalisé en s'inspirant de Blueprint (ou Blot, pour Unity), que je considère personnellement comme plus ergonomique. Il est également possible de facilement modifier le compilateur pour en faire un langage spécialisé dans la création de jeux vidéo (comme Scratch), dans l'automatisation (comme Node Red ou Tasker) et bien plus encore.

## Screenshots

![Simple Hello World](https://github.com/RaphaelNJ/vs_rs/assets/52333330/8e14e8d1-ad98-4f19-a009-fcbc0f1fd7a4)
![Demande le nom de l'utilisateur pour lui redonner ensuite](https://github.com/RaphaelNJ/vs_rs/assets/52333330/4e50c826-99e0-4867-bad6-d8e6c3869c2d)
![Démostration des fonctions personalisés](https://github.com/RaphaelNJ/vs_rs/assets/52333330/ad6518a0-741d-4091-9cfa-f6b8092e8576)

## Installation

1. Clonez le dépôt :

```shell
git clone https://github.com/RaphaelNJ/vs_rs.git
```

2. Accédez au répertoire du projet :

```shell
cd vs_rs
```

3. Executez le projet avec Cargo :

```shell
cargo run
```

## Avencement

- [x] Sauvegarder le projet dans un fichier (boutton en haut a droite)
- [x] Charger un projet depuis un fichier (boutton en haut a droite)
- [x] Compilateur basique
- [x] Support de l'execution en branches (if/else, while)
- [ ] Support de Linux (Wayland)
- [ ] Support du WebAssembly (pouvoir tourner dans un navigateur)
- [x] Créer des fonctions personalisées
- [x] Créer des arguments et des return de fonctions
- [ ] Créer un node "Return"
- [x] Variables
- [ ] Acceder au variables dans le code

## Licence

Ce programme est distribué sous la [licence MIT](https://opensource.org/licenses/MIT).
