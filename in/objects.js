// Object literal and property access
const person = {
    name: "Alice",
    age: 30,
    greet: function() {
        return "Hello, " + this.name;
    }
};

function updatePerson() {
    person.age = 31;
}

function getPersonName() {
    return person.name;
}

function getPersonAge() {
    return person.age;
}

function computedAccess() {
    const key = "name";
    const result = person[key];
    return result;  // returns "Alice" (string)
}

// Object spread operator tests
function cloneWithSpread() {
    const base = { x: 10, y: 20 };
    const copy = { ...base };
    return copy.x + copy.y;
}

function overrideWithSpread() {
    const base = { x: 10, y: 20 };
    const withOverride = { ...base, x: 99 };
    return withOverride.x + withOverride.y;
}

export { getPersonName, getPersonAge, computedAccess, updatePerson, cloneWithSpread, overrideWithSpread };

// Expected-value variables for Zig test generation
const test_getPersonName_objects = getPersonName();
const test_getPersonAge_objects = getPersonAge();
const test_computedAccess_objects = computedAccess();
const test_cloneWithSpread_objects = cloneWithSpread();
const test_overrideWithSpread_objects = overrideWithSpread();
const test_updatePerson_objects = () => updatePerson();
