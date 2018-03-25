let gulp = require("gulp");
let rename = require("gulp-rename");
let uglify = require("gulp-uglify-es").default;
let cleanCSS = require("gulp-clean-css");
let htmlmin = require('gulp-htmlmin');

gulp.task("minify-js", function () {
    return gulp.src("webapp/script.js")
        .pipe(rename("script.min.js"))
        .pipe(uglify({compress: true, toplevel: true}))
        .pipe(gulp.dest("dist/"));
});

gulp.task("minify-css", () => {
  return gulp.src("webapp/styles.css")
    .pipe(rename("styles.min.css"))
    .pipe(cleanCSS({compatibility: "*"}))
    .pipe(gulp.dest("dist/"));
});

gulp.task('minify-html', function() {
  return gulp.src('webapp/index.html')
    .pipe(rename("index.min.html"))
    .pipe(htmlmin({collapseWhitespace: true}))
    .pipe(gulp.dest('dist/'));
});

gulp.task("default", gulp.parallel("minify-js", "minify-css", "minify-html"));
