/* License: GNU GPLv3+, Rodrigo Schwencke (Copyleft) */
/* src: 
  https://github.com/rod2ik/cdn/blob/main/mkdocs/javascripts/massiliaGraphviz.js 
*/

/* cspell: words Schwencke */

window.addEventListener('load', function() {
    // console.log("massilia-graphviz PAGE LOADED");
  
    var span = document.querySelector("[class='graphviz-light-dark']:first-of-type");
    if (span !== null) { // at least one graphviz diagram in the page
      var dataLibraryDefault = span.getAttribute("data-library-default");
      var dataDefault = span.getAttribute("data-default");
      var dataLight = span.getAttribute("data-light");
      if ((dataLight == '#0')) {
        dataLight="#000000";
      }
      var dataDark = span.getAttribute("data-dark");
      tag_default();
      update_theme();
    } // else no graphviz diagram
  
    function tag_default() {
      // console.log("TAG DEFAULTS");
      if (dataDefault == dataLibraryDefault) { // if 'color' option has not been set in 'mkdocs.yml'
        document.querySelectorAll("svg.graphviz *[stroke*='"+dataDefault+"' i]")
        .forEach( el => {
          el.classList.add("stroke-default");
        });
        document.querySelectorAll("svg.graphviz *[fill*='"+dataDefault+"' i]")
        .forEach( el => {
          el.classList.add("fill-default");
        });
      }
    }
  
    function update_theme() {
      // console.log("UPDATE THEME...");
      let theme = document.querySelector("body").getAttribute("data-md-color-scheme");
      if (dataDefault == dataLibraryDefault) { // if 'color' option has not been set in 'mkdocs.yml'
        document.querySelectorAll("svg.graphviz *[class*='stroke-default']")
        .forEach( el => {
          if (theme == "default") {
            el.style.setProperty("stroke", ''+dataLight,"important");
          } else {
            el.style.setProperty("stroke", dataDark,"important");
          }
        });
  
        document.querySelectorAll("svg.graphviz *[class*='fill-default']")
        .forEach( el => {
          if (theme == "default") {
            el.style.setProperty("fill", ''+dataLight,"important");
          } else {
            el.style.setProperty("fill", dataDark,"important");
          }
        });
      }
    // other_function_to_update();
    }
  
    const mutationCallback = (mutationsList) => {
      for (const mutation of mutationsList) {
        if (
          mutation.type !== "attributes" &&
          mutation.attributeName !== "data-md-color-scheme"
        ) {
          return
        }
        update_theme();
      }
    };
  
    const observer = new MutationObserver(mutationCallback);
  
    let themeChange = document.querySelector("body");
    observer.observe(themeChange, { attributes: true });
  
  })