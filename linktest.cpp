#include <iostream>
#include <inttypes.h>
#include <chrono>

extern "C"
{
    int64_t inc(int64_t);
    int64_t sum(int64_t, int64_t);

    struct Point
    {
        double x;
        double y;
    };
    double get_x(Point);
    double get_y(Point);

    double fib(double);
}

int main()
{
    std::cout << inc(5) << std::endl;
    std::cout << sum(4, 5) << std::endl;

    auto start = std::chrono::high_resolution_clock::now();
    float fib_res = fib(35);
    auto stop = std::chrono::high_resolution_clock::now();

    auto duration = std::chrono::duration_cast<std::chrono::milliseconds>(stop - start);

    std::cout << "fib(35) = " << (long)fib(35) << " in " << duration.count() << "ms" << std::endl;

    auto point = Point{2, 5};
    std::cout << "Point { x: " << get_x(point) << ", y: " << get_y(point)
              << " }" << std::endl;
}