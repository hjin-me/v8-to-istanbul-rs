(() => {
  const isTrue = window.location.href.includes("https") ? "https" : "http";
  if (isTrue) {
    console.log(true);
  }

  const a = isTrue || 0;
  const b = isTrue && 0;
  const c = isTrue ?? 0;
  const d = isTrue.unknown?.toString();

  const ran = Math.random();
  let s;
  if (ran < 0.3) {
    s = "1";
  } else if (ran < 0.5) {
    s = "2";
  } else {
    s = "3";
  }

  switch (s) {
    case "1": {
      console.log(1);
      break;
    }
    case "2":
      console.log(2);
      console.log(2.1);
      console.log(2.2);
    case "3": {
      console.log(3);
      break;
    }
    default: {
      console.log("default");
    }
  }
})();
