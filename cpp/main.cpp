#include <iostream>
#include <cstdlib>
#include <ctime>
#include <thread>
#include <chrono>
#include <cmath>
#include <vector>
#include <algorithm>
#include <fstream> // For high score file

// Rust functions (extern "C")
extern "C" {
    struct Enemy {
        int id;
        float x;
        float y;
        bool alive;
    };

    int find_nearest_enemy(float player_x, float player_y, Enemy* enemies, int count);
    void shoot_enemy(int index, Enemy* enemies);
    void move_enemies_randomly(Enemy* enemies, int count, float speed);
}

// ANSI colors
#define RESET   "\033[0m"
#define RED     "\033[31m"
#define GREEN   "\033[32m"
#define YELLOW  "\033[33m"
#define CYAN    "\033[36m"
#define MAGENTA "\033[35m"

// Game constants
const float PLAYER_SPEED = 2.0;
const float SHOOT_RANGE = 50.0;
const int GRID_SIZE = 20;
const std::string HIGH_SCORE_FILE = "highscore.txt";

// ================= High Score Handling =================
int load_high_score() {
    std::ifstream file(HIGH_SCORE_FILE);
    int score = 0;
    if(file.is_open()) {
        file >> score;
        file.close();
    }
    return score;
}

void save_high_score(int score) {
    std::ofstream file(HIGH_SCORE_FILE);
    if(file.is_open()) {
        file << score;
        file.close();
    }
}

// ================= Menu =================
void display_menu() {
    std::cout << MAGENTA << "================ DeadAim ================\n" << RESET;
    std::cout << CYAN << "1. Start Game\n";
    std::cout << "2. View High Score\n";
    std::cout << "3. Quit\n" << RESET;
    std::cout << "Enter your choice: ";
}

void show_high_score() {
    int high_score = load_high_score();
    std::cout << GREEN << "Current High Score: " << high_score << RESET << "\n";
    std::cout << "Press enter to return to menu...";
    std::cin.ignore();
    std::cin.get();
}

// ================= Game Map =================
void draw_map(float player_x, float player_y, std::vector<Enemy>& enemies) {
    for(int y=0; y<GRID_SIZE; y++){
        for(int x=0; x<GRID_SIZE; x++){
            bool drawn = false;

            if(int(player_x)==x && int(player_y)==y){
                std::cout << GREEN << "P" << RESET;
                drawn = true;
            }

            for(auto& e: enemies){
                if(e.alive && int(e.x)==x && int(e.y)==y){
                    std::cout << RED << "E" << RESET;
                    drawn = true;
                    break;
                }
            }

            if(!drawn) std::cout << YELLOW << "." << RESET;
        }
        std::cout << "\n";
    }
}

// ================= Game =================
void play_game() {
    srand(time(0));

    int base_enemy_count = 10;
    int score = 0, health = 10, level = 1, multiplier = 1;
    int high_score = load_high_score();

    float player_x = GRID_SIZE/2;
    float player_y = GRID_SIZE/2;

    std::vector<Enemy> enemies;

    auto spawn_enemies = [&](int count){
        enemies.clear();
        for(int i=0;i<count;i++){
            Enemy e;
            e.id = i;
            e.x = rand() % GRID_SIZE;
            e.y = rand() % GRID_SIZE;
            e.alive = true;
            enemies.push_back(e);
        }
    };

    spawn_enemies(base_enemy_count);

    char input;
    while(health>0){
        std::cout << "\033[2J\033[1;1H"; // Clear screen

        // HUD
        std::cout << CYAN << "Level: " << level 
                  << "  Health: " << health 
                  << "  Score: " << score 
                  << "  Multiplier: x" << multiplier 
                  << "  High Score: " << high_score << RESET << "\n";

        draw_map(player_x, player_y, enemies);

        std::cout << "Move: W/A/S/D, Shoot: s, Quit: q >> ";
        std::cin >> input;

        if(input=='q') break;

        // Movement
        if(input=='w' && player_y>0) player_y -= PLAYER_SPEED;
        if(input=='a' && player_x>0) player_x -= PLAYER_SPEED;
        if(input=='s' && player_y<GRID_SIZE-1) player_y += PLAYER_SPEED;
        if(input=='d' && player_x<GRID_SIZE-1) player_x += PLAYER_SPEED;

        // Move enemies (Rust)
        move_enemies_randomly(enemies.data(), enemies.size(), 0.5f + 0.2f*level);

        // Nearest enemy
        int nearest = find_nearest_enemy(player_x, player_y, enemies.data(), enemies.size());

        if(nearest!=-1){
            float dx = player_x - enemies[nearest].x;
            float dy = player_y - enemies[nearest].y;
            float dist = std::sqrt(dx*dx + dy*dy);

            if(input=='s' && dist <= SHOOT_RANGE){
                shoot_enemy(nearest, enemies.data());
                std::cout << GREEN << "Shot enemy id: " << nearest << "!" << RESET << "\n";
                score += 10 * multiplier;
                multiplier++;
            } else if(dist <= 1.0){
                enemies[nearest].alive = false;
                health--;
                multiplier=1;
                std::cout << RED << "Enemy " << nearest << " hit you! Health -1" << RESET << "\n";
            }
        }

        // Level up
        bool all_dead = std::all_of(enemies.begin(), enemies.end(), [](Enemy e){ return !e.alive; });
        if(all_dead){
            level++;
            int new_enemy_count = base_enemy_count + level*5;
            spawn_enemies(new_enemy_count);
            std::cout << YELLOW << "Level " << level << " starts with " << new_enemy_count << " enemies!" << RESET << "\n";
        }

        std::this_thread::sleep_for(std::chrono::milliseconds(150));
    }

    // Game Over
    std::cout << RED << "\nGame Over! Final Score: " << score << RESET << "\n";
    if(score > high_score){
        high_score = score;
        save_high_score(high_score);
        std::cout << GREEN << "New High Score: " << high_score << "!" << RESET << "\n";
    } else {
        std::cout << CYAN << "High Score remains: " << high_score << RESET << "\n";
    }

    std::cout << "Press enter to return to menu...";
    std::cin.ignore();
    std::cin.get();
}

// ================= Main =================
int main() {
    while(true){
        std::cout << "\033[2J\033[1;1H"; // Clear screen
        display_menu();

        int choice;
        std::cin >> choice;

        switch(choice){
            case 1:
                play_game();
                break;
            case 2:
                show_high_score();
                break;
            case 3:
                std::cout << "Thanks for playing DeadAim!\n";
                return 0;
            default:
                std::cout << "Invalid choice! Try again.\n";
                std::this_thread::sleep_for(std::chrono::milliseconds(500));
        }
    }
    return 0;
}