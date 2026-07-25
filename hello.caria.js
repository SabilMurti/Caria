function main() {
    const name = `Murtix`;
    let count = 0;
    while (count < 3) {
console.log(name);
        count = count + 1;
    }
    const user = new User({ name: `Sabil`, age: 17 });
console.log(user.name);
}
class User {
    constructor({ name, age }) {
        this.name = name;
        this.age = age;
    }
}


main();